//! The agents' tools (docs/adr/8056 §2): one small surface for an agent working on a drawing, whether it comes through
//! the MCP server or chats in the desktop. A [`Host`] answers the drawing's requests by name (`crate::rpc`'s methods:
//! `summary`, `layers`, `entities`, `entity`, `measure`, `call`, and `selection`); the tools are built on those alone,
//! so the server's own drawings, a desktop's drawing on its link and the desktop's chat run the same tools alike.
//!
//! - The catalog is found step by step: `catalog.categories` (each part's families and counts), `catalog.search` (a
//!   query's commands, a line each), `catalog.describe` (one command's whole input schema). An agent never carries
//!   the five hundred schemas at once.
//! - A command runs through its part's door: `cad.run` (the core's, every module's), `arch.run`, `map.run`, `gis.run`
//!   and `road.run`; `command.batch` runs steps as one undo step (`cad.changeset.apply`). A command another part
//!   answers is sent to its door, a command the host does not offer is refused (`not_offered`, docs/adr/8056 §1).
//! - The checks and the processing tools have doors of their own, for finding them: `audit.list_providers` (every
//!   `*.check` and `*.audit` command and the repair that fixes its findings), `audit.run`, `audit.apply_fix`;
//!   `processing.list`, `processing.describe`, `processing.run`, `processing.batch` (İşlemler, docs/adr/0508).
//! - **The owner's guard (13 November 2026):** a command that writes is planned first (`mode: "plan"`: what it would
//!   add, change and remove, and the plan's id); `mode: "execute"` takes that id and writes only that plan, and only
//!   when the drawing has not changed since (the protocol's `expected.plan` and `expected.revision`, docs/adr/0253 §2).
//!   A plan is taken once. A command that only reads runs at once.

use std::collections::HashMap;
use std::sync::OnceLock;

use kentos_commands::{CommandDescriptor, CommandEffect};
use kentos_native_application::registry::{Discipline, discipline_of};
use serde_json::{Map, Value, json};

use crate::HeadlessError;

/// What the tools run on: a drawing's requests, and what the host offers.
pub trait Host {
    /// A request of the drawing by name, answered as an agent may see it (docs/adr/0738 §1).
    fn request(&mut self, method: &str, params: &Value) -> Result<Value, HeadlessError>;

    /// Why this host does not run `command` (a module's program, docs/adr/8056 §1); none when it does.
    fn not_offered(&self, _command: &str) -> Option<String> {
        None
    }
}

/// Commands that change or take away what is there (the MCP tools' `destructiveHint`); the rest add. Deleting, editing,
/// moving and setting, a block's changes, cutting and rounding, a processing tool's write.
pub const DESTRUCTIVE: &[&str] = &[
    "cad.entities.delete",
    "cad.entities.edit",
    "cad.entities.transform",
    "cad.entities.set",
    // A topology repair recipe changes and deletes the findings' objects (docs/adr/1260 §4).
    "cad.topology.repair",
    "cad.blocks.define",
    "cad.blocks.edit",
    // It moves inserts to another block and may purge the first (docs/adr/0317 §3).
    "cad.blocks.replace",
    // It writes the inserts' parameter values (docs/adr/0318 §3).
    "cad.blocks.parameters",
    "cad.blocks.freeze",
    // deleteSource deletes the objects it offsets (docs/adr/0246).
    "cad.entities.offset",
    // It binds or frees a dimension's points (docs/adr/0249 §3).
    "cad.dimensions.reassociate",
    "cad.dimensions.edit",
    "cad.fields.edit",
    "cad.hatches.edit",
    "cad.leaders.edit",
    "cad.tables.edit",
    "cad.texts.edit",
    // It takes an object's scales away (docs/adr/0323 §4).
    "cad.annotations.scales",
    // They cut, grow or round what is there; a trim takes objects away (docs/adr/0250).
    "cad.entities.trim",
    "cad.entities.extend",
    "cad.entities.fillet",
    "cad.entities.chamfer",
    // They move vertices, replace an object by its pieces and delete the joined ones (docs/adr/0254, 0255).
    "cad.entities.stretch",
    // It writes a many-to-many reference's mapping table into the file or takes it out (docs/adr/0532 §2).
    "cad.sources.relate",
    // It writes over the attributes of many objects (docs/adr/0507).
    "cad.fields.write",
    // A processing tool writes what it computes (docs/adr/0508).
    "cad.processing.run",
    "cad.processing.batch",
    // The drawing order of what is there shifts, as a set changes it (docs/adr/0260 §3, 0310).
    "cad.entities.order",
];

/// The file commands that write no file of their own: the print check and the publication's status (docs/adr/0622 §8).
const FILE_READS: &[&str] = &["file.preflight.sheets", "file.publish.status"];

/// Whether a command only reads: the catalog says it changes nothing (`effect: query`) or only the session's view, it
/// is not one that takes something away, and it writes no file.
pub fn reads_only(descriptor: &CommandDescriptor) -> bool {
    matches!(
        descriptor.effect,
        CommandEffect::Query | CommandEffect::View
    ) && !DESTRUCTIVE.contains(&descriptor.id.as_str())
        && (!descriptor.id.starts_with("file.") || FILE_READS.contains(&descriptor.id.as_str()))
}

/// A part's door: the tool, the part it runs and the part's name in a sentence.
struct Door {
    tool: &'static str,
    part: Discipline,
    words: &'static str,
}

const DOORS: [Door; 5] = [
    Door {
        tool: "cad.run",
        part: Discipline::Core,
        words: "çizimin çekirdeğinin (her modülün)",
    },
    Door {
        tool: "arch.run",
        part: Discipline::Arch,
        words: "mimarinin",
    },
    Door {
        tool: "map.run",
        part: Discipline::Map,
        words: "haritanın",
    },
    Door {
        tool: "gis.run",
        part: Discipline::Gis,
        words: "CBS'nin",
    },
    Door {
        tool: "road.run",
        part: Discipline::Road,
        words: "yolun",
    },
];

fn door_of(part: Discipline) -> &'static Door {
    DOORS.iter().find(|d| d.part == part).unwrap_or(&DOORS[0])
}

/// The part a command belongs to: the registry's part, the core's for a file command the host answers itself.
fn part_of(command: &str) -> Discipline {
    discipline_of(command).unwrap_or(Discipline::Core)
}

/// The catalog's commands an agent may run: every one a host may run, the server's own left out.
fn catalog() -> &'static [CommandDescriptor] {
    static COMMANDS: OnceLock<Vec<CommandDescriptor>> = OnceLock::new();
    COMMANDS.get_or_init(|| {
        kentos_commands::catalog::catalog()
            .commands
            .into_iter()
            .filter(|c| !matches!(c.effect, CommandEffect::Project | CommandEffect::Admin))
            .collect()
    })
}

fn descriptor(command: &str) -> Option<&'static CommandDescriptor> {
    catalog().iter().find(|c| c.id == command)
}

/// The tools `host` offers: [`tools`] without the door of a part none of whose commands it runs (a module's program,
/// docs/adr/8056 §1). The desktop's assistant offers these (§4): a door that could only answer `not_offered` costs the
/// model tokens and a wrong turn. MCP lists every door, its drawings being of every module.
pub fn offered_tools(host: &dyn Host) -> Vec<Tool> {
    tools()
        .into_iter()
        .filter(|t| match DOORS.iter().find(|d| d.tool == t.name) {
            Some(door) if door.part != Discipline::Core => catalog()
                .iter()
                .any(|c| part_of(&c.id) == door.part && host.not_offered(&c.id).is_none()),
            _ => true,
        })
        .collect()
}

/// What an agent reads first of a drawing (MCP's `kentos://session/overview`, docs/adr/8056 §5): its summary and
/// layer tree as the host lets an agent see them, and the doors the host opens.
pub fn overview(host: &mut dyn Host) -> Result<Value, HeadlessError> {
    let none = json!({});
    let summary = host.request("summary", &none)?;
    let layers = host.request("layers", &none)?;
    let doors: Vec<&str> = offered_tools(host)
        .iter()
        .filter(|t| DOORS.iter().any(|d| d.tool == t.name))
        .map(|t| t.name)
        .collect();
    Ok(json!({ "summary": summary, "layers": layers, "doors": doors }))
}

/// A tool call in words, for a person watching it (the desktop's Asistan, docs/adr/8056 §4): the command's Turkish
/// title and what the call does with it, else the tool's title and what it is asked: “Dikdörtgen duvar · plan”,
/// “Komut ara · “duvar””, “Toplu yazma · 3 adım · yazma”. Ids stay in the call's details.
pub fn call_words(name: &str, args: &Value) -> String {
    static TITLES: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    let titles = TITLES.get_or_init(|| tools().into_iter().map(|t| (t.name, t.title)).collect());
    let title = titles.get(name).copied().unwrap_or(name);
    let command = args
        .get("command")
        .and_then(Value::as_str)
        .map(|c| descriptor(c).map_or(c, |d| d.title.as_str()));
    let mode = match args.get("mode").and_then(Value::as_str) {
        Some("plan") => Some("plan"),
        Some("execute") => Some("yazma"),
        Some("validate") => Some("denetim"),
        _ => None,
    };
    let mut words: Vec<String> = Vec::new();
    match name {
        _ if DOORS.iter().any(|d| d.tool == name) => words.extend(command.map(str::to_owned)),
        "catalog.search" => {
            words.push(title.to_owned());
            words.extend(text(args, "query").ok().map(|q| format!("“{q}”")));
        }
        "command.batch" => {
            words.push(title.to_owned());
            let steps = args
                .get("steps")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            words.push(format!("{steps} adım"));
        }
        "processing.describe" | "processing.run" | "processing.batch" => {
            words.push(title.to_owned());
            words.extend(text(args, "tool").ok().map(str::to_owned));
        }
        _ => {
            words.push(title.to_owned());
            words.extend(command.map(str::to_owned));
        }
    }
    if words.is_empty() {
        words.push(title.to_owned());
    }
    words.extend(mode.map(str::to_owned));
    words.join(" · ")
}

/// The catalog's parts and families as `catalog.categories` gives them to a host of every part: MCP's
/// `kentos://catalog/categories`, read before any drawing is open (docs/adr/8056 §5).
pub fn every_category() -> Value {
    /// A host that offers everything and has no drawing.
    struct Every;
    impl Host for Every {
        fn request(&mut self, _method: &str, _params: &Value) -> Result<Value, HeadlessError> {
            Err(HeadlessError::new("no_document", "Açık çizim yok."))
        }
    }
    categories(&Every)
}

/// A tool as a host lists it: name, title, description, input schema, and whether it only reads or takes away.
pub struct Tool {
    pub name: &'static str,
    pub title: &'static str,
    pub description: String,
    pub input: Value,
    pub read_only: bool,
    pub destructive: bool,
}

/// The tools, in the order an agent meets them: the drawing, what is in it, the catalog, the doors.
pub fn tools() -> Vec<Tool> {
    let object = |properties: Value, required: &[&str]| json!({ "type": "object", "properties": properties, "required": required, "additionalProperties": false });
    let uid = json!({ "type": "string", "description": "Nesnenin kalıcı kimliği (UUID)." });
    let run_input = || {
        object(
            json!({
                "command": { "type": "string", "description": "Komutun kimliği (catalog.search ve catalog.describe verir)." },
                "input": { "type": "object", "description": "Komutun girdisi, catalog.describe'ın şemasıyla." },
                "mode": { "type": "string", "enum": ["plan", "execute", "validate"], "default": "plan",
                    "description": "plan: yazacağını gösterir, hiçbir şey yazmaz, planId verir; execute: o planı yazar \
                     (planId ister); validate: yalnız denetler. Yalnız okuyan komut hemen çalışır." },
                "planId": { "type": "string", "description": "execute için: aynı komut ve girdiyle alınan planın kimliği." }
            }),
            &["command"],
        )
    };
    let mut list = vec![
        Tool {
            name: "drawing.summary",
            title: "Çizimin özeti",
            description: "Çizimin adı, sürümü (revision), nesne sayısı, birimi, koordinat sistemi, proje türü, etkin \
                          katmanı ve geri alma durumu."
                .into(),
            input: object(json!({}), &[]),
            read_only: true,
            destructive: false,
        },
        Tool {
            name: "scene.layers",
            title: "Katmanlar",
            description: "Katman ağacı: kimlik, ad, görünürlük, kilit, renk; komutların layerId'si buradan.".into(),
            input: object(json!({}), &[]),
            read_only: true,
            destructive: false,
        },
        Tool {
            name: "scene.entities",
            title: "Nesneler",
            description: "Nesnelerin bir sayfası; katmana, türe ve kutuya (bbox: [minX, minY, maxX, maxY], projenin \
                          düzleminde metre) göre süzülür. Devamı after ile."
                .into(),
            input: object(
                json!({
                    "layer": { "type": "string" },
                    "kinds": { "type": "array", "items": { "type": "string" } },
                    "bbox": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4 },
                    "after": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 500 }
                }),
                &[],
            ),
            read_only: true,
            destructive: false,
        },
        Tool {
            name: "scene.entity",
            title: "Nesne",
            description: "Bir nesnenin bütün alanları.".into(),
            input: object(json!({ "uid": uid.clone() }), &["uid"]),
            read_only: true,
            destructive: false,
        },
        Tool {
            name: "scene.measure",
            title: "Ölç",
            description: "Bir nesnenin ölçüleri: uzunluk, alan, çevre, kotlar.".into(),
            input: object(json!({ "uid": uid }), &["uid"]),
            read_only: true,
            destructive: false,
        },
        Tool {
            name: "scene.selection",
            title: "Seçim",
            description: "Seçili nesnelerin kalıcı kimlikleri.".into(),
            input: object(json!({}), &[]),
            read_only: true,
            destructive: false,
        },
        Tool {
            name: "catalog.categories",
            title: "Komut kategorileri",
            description: "Komutların bölümleri (çekirdek, mimari, harita, CBS, yol), her bölümün kapısı (cad.run …) \
                          ve aileleri (cad.walls, cad.layers …) sayılarıyla. Keşfin ilk adımı."
                .into(),
            input: object(json!({}), &[]),
            read_only: true,
            destructive: false,
        },
        Tool {
            name: "catalog.search",
            title: "Komut ara",
            description: "Sözcüklerle komut arar (kimlik, başlık, açıklama); her sonuç bir satır: kimlik, başlık, \
                          bölüm, kapı, yazıp yazmadığı. Ardından catalog.describe."
                .into(),
            input: object(
                json!({
                    "query": { "type": "string", "description": "Arananlar, ör. \"duvar dikdörtgen\"." },
                    "part": { "type": "string", "enum": ["cad", "arch", "map", "gis", "road"] },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 50, "default": 20 }
                }),
                &["query"],
            ),
            read_only: true,
            destructive: false,
        },
        Tool {
            name: "catalog.describe",
            title: "Komutun tanımı",
            description: "Bir komutun başlığı, açıklaması, kapısı, yazıp yazmadığı, tam girdi şeması ve örnekleri."
                .into(),
            input: object(json!({ "command": { "type": "string" } }), &["command"]),
            read_only: true,
            destructive: false,
        },
    ];
    for door in &DOORS {
        list.push(Tool {
            name: door.tool,
            title: match door.part {
                Discipline::Core => "Çizim komutu çalıştır",
                Discipline::Arch => "Mimari komutu çalıştır",
                Discipline::Map => "Harita komutu çalıştır",
                Discipline::Gis => "CBS komutu çalıştır",
                Discipline::Road => "Yol komutu çalıştır",
            },
            description: format!(
                "{} komutlarından birini çalıştırır. Yazan komut önce mode: plan ile planlanır (ne eklenecek, \
                 değişecek, silinecek; planId); mode: execute aynı komut, girdi ve planId ile yalnız o planı ve \
                 yalnız çizim o andan beri değişmediyse yazar; her yazma bir geri alma adımıdır. Koordinatlar \
                 projenin düzleminde metredir.",
                capitalized(door.words)
            ),
            input: run_input(),
            read_only: false,
            destructive: true,
        });
    }
    let guarded_input = |what: &str| {
        object(
            json!({
                "input": { "type": "object", "description": what },
                "mode": { "type": "string", "enum": ["plan", "execute", "validate"], "default": "plan" },
                "planId": { "type": "string" }
            }),
            &["input"],
        )
    };
    list.push(Tool {
        name: "audit.list_providers",
        title: "Denetimler",
        description: "Çalıştırılabilecek denetimler (mimari denetim, topoloji, imar, parsel, boru, yol ekseni …): \
                      komut, başlık, kapı ve bulgularını onaran komut (varsa)."
            .into(),
        input: object(json!({}), &[]),
        read_only: true,
        destructive: false,
    });
    list.push(Tool {
        name: "audit.run",
        title: "Denetle",
        description: "Bir denetimi çalıştırır (audit.list_providers'ın komutu); bulguları verir. Yazan bir denetim \
                      önce planlanır."
            .into(),
        input: run_input(),
        read_only: false,
        destructive: false,
    });
    list.push(Tool {
        name: "audit.apply_fix",
        title: "Bulguları onar",
        description: "Bir onarım komutunu (cad.drawing.repair, cad.topology.repair, road.terrains.repair) çalıştırır: \
                      önce mode: plan, sonra planId ile execute."
            .into(),
        input: run_input(),
        read_only: false,
        destructive: true,
    });
    list.push(Tool {
        name: "extension.blueprint",
        title: "Eklenti iskeleti",
        description: "Yeni bir eklentinin çalışma klasörünü verir (dosya yolu → içerik): manifest, Python projesi ve \
                      kilidi, ikon, lisans ve istenen her katkının küçük, çalışan bir örneği (command, processing, \
                      audit, expression, mapTool). İskelet extension.verify'dan sorunsuz geçer. Hiçbir şey yazmaz: \
                      dosyaları siz yazar, örnekleri isteğe göre değiştirip doğrularsınız."
            .into(),
        input: object(
            json!({
                "id": { "type": "string", "description": "Ters alan adı, küçük harf (tr.belediye.mezarlik)" },
                "name": { "type": "string", "description": "İnsanların gördüğü ad" },
                "summary": { "type": "string", "description": "Ne yaptığını söyleyen tek cümle" },
                "kind": { "type": "string", "enum": ["vertical", "utility"], "default": "vertical",
                          "description": "vertical kendi şerit sekmesini açar, utility KentOS'unkilere araç ekler" },
                "module": { "type": "string", "enum": ["cbs", "harita", "mimari", "yol"], "default": "cbs" },
                "contributions": { "type": "array", "items": { "type": "string",
                    "enum": ["command", "processing", "audit", "expression", "mapTool"] } }
            }),
            &["id", "name", "contributions"],
        ),
        read_only: true,
        destructive: false,
    });
    list.push(Tool {
        name: "extension.verify",
        title: "Eklentiyi doğrula",
        description: "Bir eklentinin çalışma klasörünü (ya da .kek paketini) çalıştırmadan doğrular (kentos kek verify): \
                      paketin kuralları, manifestin başvuruları, Python işlevlerinin @ext işaretleri; her bulgu dosyası, \
                      satırı ya da JSON işaretçisi ve ne yazılacağıyla. Bu bilgisayardaki yolu okur, hiçbir şey yazmaz."
            .into(),
        input: object(
            json!({ "path": { "type": "string", "description": "Eklentinin klasörü ya da .kek dosyası, mutlak yol" } }),
            &["path"],
        ),
        read_only: true,
        destructive: false,
    });
    list.push(Tool {
        name: "processing.list",
        title: "İşlem araçları",
        description: "İşlemler'in araçları (tampon, kesişim, birleştirme …), parametreleriyle (cad.processing.tools)."
            .into(),
        input: object(json!({}), &[]),
        read_only: true,
        destructive: false,
    });
    list.push(Tool {
        name: "processing.describe",
        title: "İşlem aracının tanımı",
        description: "Bir İşlemler aracının parametreleri ve çıktıları.".into(),
        input: object(
            json!({ "tool": { "type": "string", "description": "Aracın kimliği." } }),
            &["tool"],
        ),
        read_only: true,
        destructive: false,
    });
    list.push(Tool {
        name: "processing.run",
        title: "İşlem aracını çalıştır",
        description: "Bir İşlemler aracını çalıştırır (cad.processing.run'ın girdisiyle); sonucunu çizime yazar: önce \
                      mode: plan, sonra planId ile execute."
            .into(),
        input: guarded_input("cad.processing.run'ın girdisi."),
        read_only: false,
        destructive: true,
    });
    list.push(Tool {
        name: "processing.batch",
        title: "İşlem araçlarını sırayla çalıştır",
        description: "İşlemler araçlarını sırayla çalıştırır (cad.processing.batch'in girdisiyle): önce plan, sonra \
                      planId ile execute."
            .into(),
        input: guarded_input("cad.processing.batch'in girdisi."),
        read_only: false,
        destructive: true,
    });
    list.push(Tool {
        name: "command.batch",
        title: "Toplu yazma",
        description: "Birden çok komutu tek geri alma adımında yazar (cad.changeset.apply): adımlar sırayla, bir \
                      adımın çıktısına sonraki adım adıyla başvurabilir. Önce mode: plan, sonra planId ile execute."
            .into(),
        input: object(
            json!({
                "steps": { "type": "array", "minItems": 1, "items": { "type": "object",
                    "properties": { "command": { "type": "string" }, "input": { "type": "object" }, "name": { "type": "string" } },
                    "required": ["command", "input"] } },
                "label": { "type": "string", "description": "Geri alma adımının adı." },
                "mode": { "type": "string", "enum": ["plan", "execute", "validate"], "default": "plan" },
                "planId": { "type": "string" }
            }),
            &["steps"],
        ),
        read_only: false,
        destructive: true,
    });
    list
}

fn capitalized(words: &str) -> String {
    let mut chars = words.chars();
    match chars.next() {
        Some('ç') => format!("Ç{}", chars.as_str()),
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The plans an agent was shown (the owner's guard): a plan's id → its command, its input and the drawing's revision
/// when it was planned. A plan is taken once; the oldest go when there are many.
#[derive(Default)]
pub struct Guard {
    plans: HashMap<String, Planned>,
    order: Vec<String>,
}

struct Planned {
    command: String,
    input: String,
    revision: String,
}

/// How many plans a guard keeps.
const PLANS_KEPT: usize = 64;

impl Guard {
    pub fn new() -> Self {
        Self::default()
    }

    fn remember(&mut self, plan: &str, command: &str, input: &Value, revision: &str) {
        if self.plans.len() >= PLANS_KEPT && !self.order.is_empty() {
            let oldest = self.order.remove(0);
            self.plans.remove(&oldest);
        }
        self.order.push(plan.to_owned());
        self.plans.insert(
            plan.to_owned(),
            Planned {
                command: command.to_owned(),
                input: canonical(input),
                revision: revision.to_owned(),
            },
        );
    }

    /// The revision a plan was made at, the plan taken; refused when it was not shown for this command and input.
    fn take(&mut self, plan: &str, command: &str, input: &Value) -> Result<String, HeadlessError> {
        let Some(planned) = self.plans.get(plan) else {
            return Err(HeadlessError::new(
                "plan_required",
                format!(
                    "“{plan}” bu oturumda gösterilmiş bir plan değil ya da kullanıldı: önce aynı komutu ve girdiyi \
                     mode: \"plan\" ile planlayın, sonra o planId ile execute edin."
                ),
            ));
        };
        if planned.command != command || planned.input != canonical(input) {
            return Err(HeadlessError::new(
                "plan_required",
                format!(
                    "“{plan}” başka bir komutun ya da girdinin planı: yazılacak komutu ve girdiyi yeniden planlayın."
                ),
            ));
        }
        self.order.retain(|p| p != plan);
        Ok(self
            .plans
            .remove(plan)
            .map(|p| p.revision)
            .unwrap_or_default())
    }
}

/// A JSON value with its objects' keys in order: the same input written in another order is the same input.
fn canonical(value: &Value) -> String {
    fn sorted(value: &Value) -> Value {
        match value {
            Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                let mut out = Map::new();
                for k in keys {
                    out.insert(k.clone(), sorted(&map[k]));
                }
                Value::Object(out)
            }
            Value::Array(items) => Value::Array(items.iter().map(sorted).collect()),
            other => other.clone(),
        }
    }
    sorted(value).to_string()
}

fn text<'a>(args: &'a Value, name: &str) -> Result<&'a str, HeadlessError> {
    args.get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| HeadlessError::new("invalid_input", format!("{name} gerekli (metin).")))
}

/// Runs tool `name` with `args` on the host's drawing, the guard keeping the plans it shows.
pub fn call(
    host: &mut dyn Host,
    guard: &mut Guard,
    name: &str,
    args: &Value,
) -> Result<Value, HeadlessError> {
    let none = json!({});
    match name {
        "drawing.summary" => host.request("summary", &none),
        "scene.layers" => host.request("layers", &none),
        "scene.entities" => host.request("entities", args),
        "scene.entity" => host.request("entity", args),
        "scene.measure" => host.request("measure", args),
        "scene.selection" => host.request("selection", &none),
        "catalog.categories" => Ok(categories(host)),
        "catalog.search" => search(host, args),
        "catalog.describe" => describe(host, args),
        "command.batch" => batch(host, guard, args),
        "audit.list_providers" => Ok(providers(host)),
        "audit.run" => audit(host, guard, args, false),
        "audit.apply_fix" => audit(host, guard, args, true),
        "processing.list" => processing_tools(host),
        "extension.verify" => extension_verify(args),
        "extension.blueprint" => extension_blueprint(args),
        "processing.describe" => {
            let tool = text(args, "tool")?;
            let tools = processing_tools(host)?;
            find_processing_tool(&tools, tool).cloned().ok_or_else(|| {
                HeadlessError::new(
                    "unknown_tool",
                    format!("“{tool}” diye bir işlem aracı yok: processing.list."),
                )
            })
        }
        "processing.run" => processing(host, guard, args, "cad.processing.run"),
        "processing.batch" => processing(host, guard, args, "cad.processing.batch"),
        tool => match DOORS.iter().find(|d| d.tool == tool) {
            Some(door) => run(host, guard, door, args),
            None => Err(HeadlessError::new(
                "unknown_tool",
                format!("“{tool}” diye bir araç yok."),
            )),
        },
    }
}

/// A command through its door: its part's, offered by the host, planned before it writes.
fn run(
    host: &mut dyn Host,
    guard: &mut Guard,
    door: &Door,
    args: &Value,
) -> Result<Value, HeadlessError> {
    let command = text(args, "command")?;
    let Some(descriptor) = descriptor(command) else {
        return Err(HeadlessError::new(
            "unknown_command",
            format!("“{command}” katalogda yok: catalog.search ile arayın."),
        ));
    };
    let part = part_of(command);
    if part != door.part {
        let right = door_of(part);
        return Err(HeadlessError::new(
            "wrong_tool",
            format!(
                "“{command}” {} komutu: {} ile çağırın.",
                right.words, right.tool
            ),
        ));
    }
    if let Some(why) = host.not_offered(command) {
        return Err(HeadlessError::new("not_offered", why));
    }
    let input = args.get("input").cloned().unwrap_or_else(|| json!({}));
    guarded(host, guard, command, reads_only(descriptor), input, args)
}

/// Steps as one undo step (`cad.changeset.apply`), each step's command offered by the host.
fn batch(host: &mut dyn Host, guard: &mut Guard, args: &Value) -> Result<Value, HeadlessError> {
    let steps = args
        .get("steps")
        .and_then(Value::as_array)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| HeadlessError::new("invalid_input", "steps en az bir adım ister."))?;
    for step in steps {
        let command = text(step, "command")?;
        if descriptor(command).is_none() {
            return Err(HeadlessError::new(
                "unknown_command",
                format!("“{command}” katalogda yok."),
            ));
        }
        if let Some(why) = host.not_offered(command) {
            return Err(HeadlessError::new("not_offered", why));
        }
    }
    let mut input = json!({ "steps": steps });
    if let Some(label) = args.get("label").filter(|l| l.is_string()) {
        input["label"] = label.clone();
    }
    guarded(host, guard, "cad.changeset.apply", false, input, args)
}

/// The owner's guard around a command's call: a read runs at once; a write is planned, then executed only as planned.
fn guarded(
    host: &mut dyn Host,
    guard: &mut Guard,
    command: &str,
    reads: bool,
    input: Value,
    args: &Value,
) -> Result<Value, HeadlessError> {
    let mode = args.get("mode").and_then(Value::as_str).unwrap_or("plan");
    let call = |op: &str, input: &Value| json!({ "command": command, "op": op, "input": input });
    match (mode, reads) {
        ("validate", _) => host.request("call", &call("validate", &input)),
        ("plan" | "execute", true) => host.request("call", &call("execute", &input)),
        ("plan", false) => {
            let revision = host.request("summary", &json!({}))?["revision"]
                .as_str()
                .unwrap_or_default()
                .to_owned();
            let mut answer = host.request("call", &call("plan", &input))?;
            if let Some(plan) = answer
                .get("planId")
                .and_then(Value::as_str)
                .map(str::to_owned)
            {
                guard.remember(&plan, command, &input, &revision);
                answer["next"] = json!(format!(
                    "Yazmak için aynı komut ve girdiyle mode: \"execute\", planId: \"{plan}\". Çizim bu arada \
                     değişirse plan geçersizdir."
                ));
            }
            Ok(answer)
        }
        ("execute", false) => {
            let plan = args.get("planId").and_then(Value::as_str).ok_or_else(|| {
                HeadlessError::new(
                    "plan_required",
                    "Yazmadan önce plan: aynı komutu ve girdiyi mode: \"plan\" ile çağırın, sonra o planId ile \
                     execute edin.",
                )
            })?;
            let revision = guard.take(plan, command, &input)?;
            let mut request = call("execute", &input);
            request["expected"] = json!({ "plan": plan, "revision": revision });
            host.request("call", &request)
        }
        (other, _) => Err(HeadlessError::new(
            "invalid_input",
            format!("mode “{other}” olamaz: plan, execute ya da validate."),
        )),
    }
}

/// Turkish lower case, as a query and the catalog's words are compared.
fn lower(text: &str) -> String {
    text.replace('I', "ı").replace('İ', "i").to_lowercase()
}

fn part_id(part: Discipline) -> &'static str {
    match part {
        Discipline::Core => "cad",
        Discipline::Arch => "arch",
        Discipline::Map => "map",
        Discipline::Gis => "gis",
        Discipline::Road => "road",
    }
}

/// A command's family: its id's first two words (`cad.walls`).
fn family(id: &str) -> &str {
    match id.match_indices('.').nth(1) {
        Some((at, _)) => &id[..at],
        None => id,
    }
}

/// Each part's door, families and counts, of the commands the host offers.
fn categories(host: &dyn Host) -> Value {
    let mut parts: Vec<Value> = Vec::new();
    for door in &DOORS {
        let mut families: Vec<(String, usize)> = Vec::new();
        for c in catalog()
            .iter()
            .filter(|c| part_of(&c.id) == door.part && host.not_offered(&c.id).is_none())
        {
            let f = family(&c.id);
            match families.iter_mut().find(|(name, _)| name == f) {
                Some((_, n)) => *n += 1,
                None => families.push((f.to_owned(), 1)),
            }
        }
        let count: usize = families.iter().map(|(_, n)| n).sum();
        if count == 0 {
            continue;
        }
        families.sort();
        parts.push(json!({
            "part": part_id(door.part),
            "door": door.tool,
            "commands": count,
            "families": families.into_iter().map(|(f, n)| json!({ "family": f, "commands": n })).collect::<Vec<_>>(),
        }));
    }
    json!({ "parts": parts, "next": "catalog.search ile arayın, catalog.describe ile şemayı alın." })
}

/// The commands whose id, title, aliases or summary hold a word of the query: those holding the most words first,
/// then those holding them in their id or title (an agent may write a Turkish word where the id is English:
/// “çokgen alan” finds Kapalı alan oluştur by its “alan”).
fn search(host: &dyn Host, args: &Value) -> Result<Value, HeadlessError> {
    let query = lower(text(args, "query")?);
    let words: Vec<&str> = query.split_whitespace().collect();
    if words.is_empty() {
        return Err(HeadlessError::new("invalid_input", "query boş olamaz."));
    }
    let part = args.get("part").and_then(Value::as_str);
    let limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(20)
        .clamp(1, 50) as usize;
    let mut found: Vec<(usize, u32, &CommandDescriptor)> = Vec::new();
    for c in catalog() {
        if part.is_some_and(|p| p != part_id(part_of(&c.id))) || host.not_offered(&c.id).is_some() {
            continue;
        }
        let id = lower(&c.id);
        let title = lower(&c.title);
        let aliases = lower(&c.aliases.join(" "));
        let summary = lower(&c.summary);
        let mut score = 0;
        let mut held = 0;
        for w in &words {
            let s = if id.contains(w) {
                4
            } else if title.contains(w) || aliases.contains(w) {
                3
            } else if summary.contains(w) {
                1
            } else {
                0
            };
            held += usize::from(s > 0);
            score += s;
        }
        if held > 0 {
            found.push((held, score, c));
        }
    }
    found.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)).then(a.2.id.cmp(&b.2.id)));
    let total = found.len();
    let commands: Vec<Value> = found
        .into_iter()
        .take(limit)
        .map(|(_, _, c)| {
            json!({
                "command": c.id,
                "title": c.title,
                "summary": first_sentence(&c.summary),
                "door": door_of(part_of(&c.id)).tool,
                "writes": !reads_only(c),
            })
        })
        .collect();
    Ok(json!({ "commands": commands, "total": total }))
}

/// A summary's first sentence, at most 200 characters.
fn first_sentence(summary: &str) -> String {
    let end = summary.find(". ").map_or(summary.len(), |i| i + 1);
    let cut: String = summary[..end].chars().take(200).collect();
    cut
}

/// One command whole: its words, door, whether it writes, its input schema and examples.
fn describe(host: &dyn Host, args: &Value) -> Result<Value, HeadlessError> {
    let command = text(args, "command")?;
    let c = descriptor(command).ok_or_else(|| {
        HeadlessError::new(
            "unknown_command",
            format!("“{command}” katalogda yok: catalog.search ile arayın."),
        )
    })?;
    if let Some(why) = host.not_offered(command) {
        return Err(HeadlessError::new("not_offered", why));
    }
    Ok(json!({
        "command": c.id,
        "title": c.title,
        "summary": c.summary,
        "door": door_of(part_of(&c.id)).tool,
        "writes": !reads_only(c),
        "input": c.input,
        "examples": c.examples.iter().take(3).map(|e| json!({ "title": e.title, "input": e.input })).collect::<Vec<_>>(),
    }))
}

/// A check: a command that reads the drawing and gives findings (`*.check`, `*.audit`).
fn is_check(id: &str) -> bool {
    id.ends_with(".check") || id.ends_with(".audit")
}

/// The repair that fixes a check's findings, when there is one: the same family's `*.repair`
/// (`cad.topology.check` → `cad.topology.repair`), else the drawing's own (`cad.drawing.audit` → `cad.drawing.repair`).
fn repair_of(check: &str) -> Option<&'static str> {
    let checked = family(check);
    catalog()
        .iter()
        .map(|c| c.id.as_str())
        .find(|id| id.ends_with(".repair") && family(id) == checked)
}

/// The checks the host offers, each with its door and its repair.
fn providers(host: &dyn Host) -> Value {
    let checks: Vec<Value> = catalog()
        .iter()
        .filter(|c| is_check(&c.id) && host.not_offered(&c.id).is_none())
        .map(|c| {
            json!({
                "command": c.id,
                "title": c.title,
                "summary": first_sentence(&c.summary),
                "door": door_of(part_of(&c.id)).tool,
                "repair": repair_of(&c.id).filter(|r| host.not_offered(r).is_none()),
            })
        })
        .collect();
    json!({ "providers": checks, "next": "catalog.describe girdisini verir; audit.run çalıştırır, audit.apply_fix onarır." })
}

/// A check run, or a repair applied: its door's rules, its own kind only.
fn audit(
    host: &mut dyn Host,
    guard: &mut Guard,
    args: &Value,
    fix: bool,
) -> Result<Value, HeadlessError> {
    let command = text(args, "command")?;
    let right = if fix {
        command.ends_with(".repair")
    } else {
        is_check(command)
    };
    if !right {
        return Err(HeadlessError::new(
            "wrong_tool",
            if fix {
                format!(
                    "“{command}” bir onarım komutu değil: audit.list_providers onarımları söyler."
                )
            } else {
                format!("“{command}” bir denetim değil: audit.list_providers denetimleri söyler.")
            },
        ));
    }
    run(host, guard, door_of(part_of(command)), args)
}

/// İşlemler's tools as `cad.processing.tools` gives them.
fn processing_tools(host: &mut dyn Host) -> Result<Value, HeadlessError> {
    let command = "cad.processing.tools";
    if let Some(why) = host.not_offered(command) {
        return Err(HeadlessError::new("not_offered", why));
    }
    host.request(
        "call",
        &json!({ "command": command, "op": "execute", "input": {} }),
    )
}

/// A tool of `cad.processing.tools`' answer by its id, wherever the answer keeps its list.
fn find_processing_tool<'a>(answer: &'a Value, id: &str) -> Option<&'a Value> {
    let list = answer
        .pointer("/output/tools")
        .or_else(|| answer.get("tools"))
        .and_then(Value::as_array)?;
    list.iter()
        .find(|t| t.get("id").and_then(Value::as_str) == Some(id))
}

/// A processing run or batch through the guard, its command offered by the host.
fn processing(
    host: &mut dyn Host,
    guard: &mut Guard,
    args: &Value,
    command: &str,
) -> Result<Value, HeadlessError> {
    if let Some(why) = host.not_offered(command) {
        return Err(HeadlessError::new("not_offered", why));
    }
    let input = args.get("input").cloned().unwrap_or_else(|| json!({}));
    guarded(host, guard, command, false, input, args)
}

/// The folders an extension's working folder has that no package carries, not read at all (a `.venv` holds hundreds
/// of megabytes); the packer's own rules leave out the rest.
const NOT_READ: &[&str] = &[
    ".venv",
    ".git",
    ".hg",
    ".svn",
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    ".tox",
    ".nox",
    "node_modules",
    ".ipynb_checkpoints",
];

/// `extension.verify` (plan/eklenti.md ADIM 21, docs/adr/8057 §18): an extension's working folder through
/// `kentos_extensions::verify`, or a `.kek` file through the package's check; nothing runs, nothing is written.
fn extension_verify(args: &Value) -> Result<Value, HeadlessError> {
    let path = std::path::PathBuf::from(text(args, "path")?);
    let limits = kentos_extensions::Limits::default();
    let unreadable = |e: std::io::Error| {
        HeadlessError::new(
            "file_not_found",
            format!("{} okunamadı: {e}", path.display()),
        )
    };
    if path.is_file() {
        let bytes = std::fs::read(&path).map_err(unreadable)?;
        let i = kentos_extensions::inspect(&bytes, &limits);
        return Ok(json!({
            "valid": i.is_valid(),
            "extension": i.manifest.as_ref().map(|m| json!({ "id": m.id, "version": m.version.to_string(), "name": m.name })),
            "digest": i.digest,
            "problems": i.problems,
            "warnings": i.warnings,
            "next": "Paketin kaynağını doğrulamak için klasörünü verin: Python işaretleri de denetlenir.",
        }));
    }
    let mut files = std::collections::BTreeMap::new();
    let mut stack = vec![path.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).map_err(unreadable)? {
            let entry = entry.map_err(unreadable)?;
            let at = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if at.is_dir() {
                if !NOT_READ.contains(&name.as_str()) {
                    stack.push(at);
                }
                continue;
            }
            if files.len() >= limits.entries {
                return Err(HeadlessError::new(
                    "input_too_large",
                    format!(
                        "Klasörde {} dosyadan fazlası var: bir paket bu kadarını taşımaz.",
                        limits.entries
                    ),
                ));
            }
            let Some(key) = at
                .strip_prefix(&path)
                .ok()
                .and_then(|p| p.to_str())
                .map(|p| p.replace('\\', "/"))
            else {
                continue;
            };
            files.insert(key, std::fs::read(&at).map_err(unreadable)?);
        }
    }
    let v = kentos_extensions::verify(&files, &limits);
    Ok(json!({
        "valid": v.is_valid(),
        "extension": v.manifest.as_ref().map(|m| json!({ "id": m.id, "version": m.version.to_string(), "name": m.name })),
        "digest": v.digest,
        "problems": v.problems,
        "warnings": v.warnings,
        "next": if v.is_valid() {
            "Doğrulandı: `kentos kek pack` paketler; kurulum kullanıcının incelemesiyle olur."
        } else {
            "Sorunları dosya ve satırlarıyla düzeltip yeniden doğrulayın."
        },
    }))
}

/// `extension.blueprint` (plan/eklenti.md ADIM 21, docs/adr/8057 §19): a new extension's working folder as files, valid
/// by construction; nothing is written.
fn extension_blueprint(args: &Value) -> Result<Value, HeadlessError> {
    use kentos_extensions::blueprint::{Ask, Contribution, blueprint};
    use kentos_extensions::manifest::Module;
    let module = match args.get("module").and_then(Value::as_str).unwrap_or("cbs") {
        "cbs" => Module::Cbs,
        "harita" => Module::Harita,
        "mimari" => Module::Mimari,
        "yol" => Module::Yol,
        other => {
            return Err(HeadlessError::new(
                "invalid_input",
                format!("“{other}” bir program değil: cbs, harita, mimari ya da yol."),
            ));
        }
    };
    let mut contributions = Vec::new();
    for word in args
        .get("contributions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let word = word.as_str().unwrap_or_default();
        contributions.push(Contribution::parse(word).ok_or_else(|| {
            HeadlessError::new(
                "invalid_input",
                format!(
                    "“{word}” bir katkı değil: command, processing, audit, expression, mapTool."
                ),
            )
        })?);
    }
    let name = text(args, "name")?;
    let ask = Ask {
        id: text(args, "id")?.to_owned(),
        name: name.to_owned(),
        summary: args
            .get("summary")
            .and_then(Value::as_str)
            .map_or_else(|| format!("{name} eklentisi."), str::to_owned),
        vertical: args.get("kind").and_then(Value::as_str) != Some("utility"),
        module,
        contributions,
    };
    let files = blueprint(&ask).map_err(|why| HeadlessError::new("invalid_input", why))?;
    Ok(json!({
        "files": files,
        "next": "Dosyaları boş bir klasöre yazın; örnekleri isteğe göre değiştirin; her adımda extension.verify ile \
                 doğrulayın. Kütüphane gerekirse klasörde `uv add <paket>` (pyproject.toml ve uv.lock birlikte \
                 yazılır). Kurulum kullanıcının incelemesiyle olur.",
    }))
}
