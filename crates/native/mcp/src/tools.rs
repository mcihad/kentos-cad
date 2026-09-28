//! The tools (docs/adr/0133): the catalog's drawing commands under their own
//! ids, with the drawing's handle and the call's way (`op`) added to their
//! input schemas, and the drawing's own tools: open, new, save, close, list,
//! summary, layers, entities, entity, measure, undo, redo. Resources: the
//! catalog and each open drawing.

use std::path::Path;

use kentos_domain::Uuid;
use kentos_headless::{HeadlessError, NewDrawing, Op, Session};
use serde_json::{Value, json};

use crate::cloud;
use crate::desktop::{self, Desktop};
use crate::server::{Fault, Server};

/// The most drawings open at once: another is refused until one is closed.
const MOST_DRAWINGS: usize = 16;
/// A page's objects when the call names none, and at most.
const PAGE: u64 = 100;
const PAGE_MAX: u64 = 1000;

const DRAWING: &str = "Açık çizimin tutamacı (drawing.open ya da drawing.new'in verdiği).";

fn drawing_arg() -> Value {
    json!({ "type": "string", "description": DRAWING })
}

fn object(properties: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": properties, "required": required, "additionalProperties": false })
}

fn annotations(read_only: bool, destructive: bool) -> Value {
    json!({
        "readOnlyHint": read_only,
        "destructiveHint": destructive,
        "idempotentHint": read_only,
        "openWorldHint": false,
    })
}

/// A tool that reaches the KentOS server.
fn remote(read_only: bool, destructive: bool) -> Value {
    let mut notes = annotations(read_only, destructive);
    notes["openWorldHint"] = json!(true);
    notes
}

const TENANT: &str = "Projenin kurumu ya da kişisel alanı (UUID; project.list'in tenantId'si).";
const PROJECT: &str = "Projenin kimliği (UUID; project.list'in id'si).";

fn tool(name: &str, title: &str, description: &str, input: Value, notes: Value) -> Value {
    json!({ "name": name, "title": title, "description": description, "inputSchema": input, "annotations": notes })
}

/// Every tool, in a fixed order: the drawing's own, then the catalog's.
pub fn list(catalog: &Value) -> Vec<Value> {
    let uid = json!({ "type": "string", "description": "Nesnenin kalıcı kimliği (küçük harfli, tireli UUID)." });
    let mut tools = vec![
        tool(
            "drawing.open",
            "Çizim aç",
            "Bir .kcad dosyasını (KCAD v2 ya da eski v1 JSON) açar; tutamacını ve özetini verir.",
            object(
                json!({ "path": { "type": "string", "description": "Dosyanın yolu." } }),
                &["path"],
            ),
            annotations(false, false),
        ),
        tool(
            "drawing.new",
            "Yeni çizim",
            "Yeni bir proje başlatır (standart katman ağacı, dilimin çalışma alanı, varsayılan birimler); \
             koordinat sistemi sorulur, tahmin edilmez.",
            object(
                json!({
                    "name": { "type": "string", "description": "Projenin adı." },
                    "srid": { "type": "integer", "description": "Koordinat sistemi (5254…5259 TUREF/TM, 32635…32638 UTM …)." },
                    "plotScale": { "type": "number", "description": "Çizim ölçeği paydası (1000: 1:1000)." },
                    "workspace": { "type": "string", "enum": ["hybrid", "cad", "gis", "disaster"] },
                    "drawingFont": { "type": "string", "description": "Çizimin yazı tipi (barlow, arimo …)." }
                }),
                &["srid"],
            ),
            annotations(false, false),
        ),
        tool(
            "desktop.attach",
            "Masaüstündeki çizime bağlan",
            "KentOS masaüstünde açık çizime bağlanır; tutamacını verir. drawing.* ve cad.* araçları bu \
             tutamaçla ekrandaki çizimi okur ve komutlarla yazar; her yazma masaüstünde bir geri alma \
             adımıdır, kullanıcı hemen görür. Kullanıcı önce Python sekmesinden “Ajanlara aç” demelidir. \
             Kaydetmek ve geri almak kullanıcınındır.",
            object(
                json!({ "path": { "type": "string", "description": "Bağlantının yolu; yoksa masaüstünün varsayılanı." } }),
                &[],
            ),
            annotations(false, false),
        ),
        tool(
            "drawing.save",
            "Çizimi kaydet",
            "KCAD v2 olarak kaydeder: verilen yola ya da açıldığı dosyaya. Baytlar geri okunmadan dosyanın \
             yerine konmaz; eski v1 dosyasının üzerine yazılmaz.",
            object(
                json!({ "drawing": drawing_arg(), "path": { "type": "string", "description": "Nereye; yoksa açıldığı dosya." } }),
                &["drawing"],
            ),
            annotations(false, true),
        ),
        tool(
            "drawing.close",
            "Çizimi kapat",
            "Tutamacı bırakır. Kaydedilmemiş değişiklik varsa reddeder; discard: true onları atar.",
            object(
                json!({ "drawing": drawing_arg(), "discard": { "type": "boolean" } }),
                &["drawing"],
            ),
            annotations(false, true),
        ),
        tool(
            "drawing.list",
            "Açık çizimler",
            "Bu süreçte açık çizimler: tutamaç, ad, dosya, nesne sayısı, revizyon, kaydedilmemiş iş.",
            object(json!({}), &[]),
            annotations(true, false),
        ),
        tool(
            "drawing.summary",
            "Çizimin özeti",
            "Ad, revizyon, nesne sayısı, proje ayarları (SRID, birimler, ölçek), etkin katman, geri alma durumu.",
            object(json!({ "drawing": drawing_arg() }), &["drawing"]),
            annotations(true, false),
        ),
        tool(
            "drawing.layers",
            "Katman ağacı",
            "Gruplar ve katmanlar: kimlikleri, adları, görünürlük, kilit ve stilleriyle.",
            object(json!({ "drawing": drawing_arg() }), &["drawing"]),
            annotations(true, false),
        ),
        tool(
            "drawing.entities",
            "Nesneler (sayfa)",
            "Nesneleri çizimin sırasıyla sayfa sayfa verir: katman, tür ve kutu süzgeci; next imleciyle sürer. \
             Bütün çizimi bir kerede istemeyin.",
            object(
                json!({
                    "drawing": drawing_arg(),
                    "layer": { "type": "string", "description": "Yalnız bu katmandakiler (katmanın kimliği)." },
                    "kinds": { "type": "array", "items": { "type": "string" }, "description": "point, line, polyline, polygon, circle, arc, text …" },
                    "bbox": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4, "description": "[min_x, min_y, max_x, max_y]" },
                    "after": { "type": "string", "description": "Önceki sayfanın next'i." },
                    "limit": { "type": "integer", "minimum": 1, "maximum": PAGE_MAX, "default": PAGE }
                }),
                &["drawing"],
            ),
            annotations(true, false),
        ),
        tool(
            "drawing.entity",
            "Nesne",
            "Bir nesne kalıcı kimliğiyle, bütün alanlarıyla.",
            object(
                json!({ "drawing": drawing_arg(), "uid": uid }),
                &["drawing", "uid"],
            ),
            annotations(true, false),
        ),
        tool(
            "drawing.measure",
            "Ölç",
            "Nesnenin alanı (m²), uzunluğu ya da çevresi (m) ve kutusu; kaynak geometriden.",
            object(
                json!({ "drawing": drawing_arg(), "uid": uid }),
                &["drawing", "uid"],
            ),
            annotations(true, false),
        ),
        tool(
            "drawing.undo",
            "Geri al",
            "Son adımı geri alır; adının adını verir.",
            object(json!({ "drawing": drawing_arg() }), &["drawing"]),
            annotations(false, false),
        ),
        tool(
            "drawing.redo",
            "Yinele",
            "Geri alınan son adımı yineler.",
            object(json!({ "drawing": drawing_arg() }), &["drawing"]),
            annotations(false, false),
        ),
    ];
    let commands: Vec<&Value> = catalog["commands"]
        .as_array()
        .into_iter()
        .flatten()
        .collect();
    let local = |c: &Value| {
        c["hosts"]
            .as_array()
            .is_some_and(|h| h.iter().any(|x| x == "desktop"))
    };
    for c in commands.iter().filter(|c| local(c)) {
        tools.push(command_tool(c));
    }
    tools.push(tool(
        "project.list",
        "Bulut projeleri",
        "Hesabın proje kataloğundan bir sayfa: kimlik, kurum (tenantId), ad, rol, durum … \
         view: mine, organization (tenant ile), shared, recent, favorites, archived, trash. \
         Sunucuya ortamdaki hesapla bağlanır.",
        object(
            json!({
                "view": { "type": "string", "enum": ["mine", "organization", "shared", "recent", "favorites", "archived", "trash"], "default": "mine" },
                "q": { "type": "string", "description": "Adda, açıklamada ya da etiketlerde geçen sözcükler." },
                "tenant": { "type": "string", "description": TENANT },
                "limit": { "type": "integer", "minimum": 1, "maximum": 200 },
                "after": { "type": "string", "description": "Önceki sayfanın next'i." }
            }),
            &[],
        ),
        remote(true, false),
    ));
    tools.push(tool(
        "project.open",
        "Bulut projesini aç",
        "Bir veritabanı projesinin bu anki görüntüsünü (.kcad) indirir ve yerel bir çizim olarak açar; \
         tutamacını verir. Yerel kopyadır: değişiklikleri buluta gitmez, buluta project.changes yazar.",
        object(
            json!({
                "tenant": { "type": "string", "description": TENANT },
                "project": { "type": "string", "description": PROJECT }
            }),
            &["tenant", "project"],
        ),
        remote(true, false),
    ));
    for c in commands.iter().filter(|c| !local(c)) {
        tools.push(server_tool(c));
    }
    tools
}

/// A server command as a tool: its input schema with the project it acts on,
/// the versions it rests on and its idempotency key.
fn server_tool(c: &Value) -> Value {
    let id = c["id"].as_str().unwrap_or_default();
    let project_scope = c["requires"]
        .as_array()
        .is_some_and(|r| r.iter().any(|x| x == "cloudProject"));
    let mut schema = c["input"].clone();
    if let Some(fields) = schema.as_object_mut() {
        fields.remove("title");
    }
    if !schema["properties"].is_object() {
        schema["properties"] = json!({});
    }
    schema["properties"]["tenant"] = json!({ "type": "string", "description": TENANT });
    let mut own = vec![json!("tenant")];
    if project_scope {
        schema["properties"]["project"] = json!({ "type": "string", "description": PROJECT });
        own.push(json!("project"));
    }
    schema["properties"]["expectedVersions"] = json!({
        "type": "object",
        "additionalProperties": { "type": "string" },
        "description": "Değişikliğin dayandığı sürümler: @project (katalog), @file (dosya revizyonu), nesne kimlikleri."
    });
    schema["properties"]["idempotencyKey"] = json!({
        "type": "string",
        "description": "Aynı anahtarla yeniden gönderilen istek aynı sonucu alır, ikinci kez yazılmaz."
    });
    match schema["required"].as_array_mut() {
        Some(required) => {
            for (i, name) in own.into_iter().enumerate() {
                required.insert(i, name);
            }
        }
        None => schema["required"] = json!(own),
    }
    let destructive = matches!(
        id,
        "project.purge"
            | "project.trash"
            | "project.access.revoke"
            | "project.invitation.revoke"
            | "project.checkpoint.delete"
            | "project.changes"
    );
    let description = format!(
        "{}\n\nKentOS sunucusunda, ortamdaki hesabın yetkisiyle çalışır; yetkiyi sunucu denetler. \
         Sonuç komutun çıktısıdır; sunucunun reddi status, code, message ve path ile gelir.",
        c["summary"].as_str().unwrap_or_default()
    );
    json!({
        "name": id,
        "title": c["title"],
        "description": description,
        "inputSchema": schema,
        "annotations": remote(false, destructive),
    })
}

/// A catalog command as a tool: its input schema with `drawing` and `op`.
fn command_tool(c: &Value) -> Value {
    let id = c["id"].as_str().unwrap_or_default();
    let mut schema = c["input"].clone();
    if let Some(fields) = schema.as_object_mut() {
        fields.remove("title");
    }
    schema["properties"]["drawing"] = drawing_arg();
    schema["properties"]["op"] = json!({
        "type": "string",
        "enum": ["execute", "plan", "validate"],
        "default": "execute",
        "description": "execute: tek geri alma adımı olarak yazar; plan: yazılacak olanı gösterir, hiçbir şey yazmaz; validate: yalnız denetler."
    });
    match schema["required"].as_array_mut() {
        Some(required) => required.insert(0, json!("drawing")),
        None => schema["required"] = json!(["drawing"]),
    }
    // Deleting, editing, moving and setting change what is there; the rest adds.
    let destructive = matches!(
        id,
        "cad.entities.delete" | "cad.entities.edit" | "cad.entities.transform" | "cad.entities.set"
    );
    let description = format!(
        "{}\n\nSonuç CommandResult'tır: status completed (output, warnings) ya da failed, needs_input, \
         conflict (error: code, message, path). plan'ın revision'u expectedRevision olarak verilirse \
         yalnız o plan yazılır. Koordinatlar x doğu (Y), y kuzey (X), metre.",
        c["summary"].as_str().unwrap_or_default()
    );
    json!({
        "name": id,
        "title": c["title"],
        "description": description,
        "inputSchema": schema,
        "annotations": annotations(false, destructive),
    })
}

/// A tool's answer: the value as structured content and its JSON as text.
fn answer(value: Value, error: bool) -> Value {
    let text = serde_json::to_string(&value).unwrap_or_default();
    json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": value,
        "isError": error,
    })
}

/// A refusal the model can act on: its code and message.
fn refused(code: &str, message: impl Into<String>) -> Value {
    let message = message.into();
    let value = json!({ "error": { "code": code, "message": message } });
    json!({
        "content": [{ "type": "text", "text": format!("{code}: {message}") }],
        "structuredContent": value,
        "isError": true,
    })
}

fn host(e: HeadlessError) -> Value {
    refused(e.code, e.message)
}

fn text<'a>(args: &'a Value, name: &str) -> Option<&'a str> {
    args.get(name).and_then(Value::as_str)
}

/// Runs a tool; every failure the model can fix is a result with `isError`.
pub fn call(server: &mut Server, name: &str, args: &Value) -> Value {
    match run(server, name, args) {
        Ok(value) => value,
        Err(value) => value,
    }
}

fn session<'a>(server: &'a mut Server, args: &Value) -> Result<(&'a mut Session, String), Value> {
    let handle =
        text(args, "drawing").ok_or_else(|| refused("invalid_input", "drawing gerekli."))?;
    let s = server.drawings.get_mut(handle).ok_or_else(|| {
        refused(
            "unknown_drawing",
            format!("“{handle}” açık bir çizim değil: drawing.list açık olanları söyler, drawing.open açar."),
        )
    })?;
    Ok((s, handle.to_owned()))
}

fn opened(server: &mut Server, session: Session) -> Result<Value, Value> {
    if server.drawings.len() >= MOST_DRAWINGS {
        return Err(refused(
            "busy",
            format!(
                "En çok {MOST_DRAWINGS} çizim açık olabilir: drawing.close ile birini kapatın."
            ),
        ));
    }
    let handle = Uuid::new_v4().to_string();
    let summary = session.summary();
    server.drawings.insert(handle.clone(), session);
    Ok(answer(
        json!({ "drawing": handle, "summary": summary }),
        false,
    ))
}

/// A handle on the desktop's drawing: its tools go there.
fn on_desktop(server: &mut Server, handle: &str, name: &str, args: &Value) -> Result<Value, Value> {
    let (method, params) = match name {
        "drawing.summary" => ("summary", json!({})),
        "drawing.layers" => ("layers", json!({})),
        "drawing.entities" => (
            "entities",
            json!({
                "layer": args.get("layer"),
                "kinds": args.get("kinds"),
                "bbox": args.get("bbox"),
                "after": args.get("after"),
                "limit": args.get("limit").and_then(Value::as_u64).unwrap_or(PAGE).clamp(1, PAGE_MAX),
            }),
        ),
        "drawing.entity" => ("entity", json!({ "uid": args.get("uid") })),
        "drawing.measure" => ("measure", json!({ "uid": args.get("uid") })),
        "drawing.close" => {
            server.desktops.remove(handle);
            return Ok(answer(json!({ "closed": handle, "detached": true }), false));
        }
        "drawing.save" | "drawing.undo" | "drawing.redo" => {
            return Err(refused(
                "desktop_user",
                "Masaüstündeki çizimi kullanıcı kaydeder ve geri alır: ondan isteyin.",
            ));
        }
        command if command.starts_with("cad.") => {
            let mut input = args.clone();
            if let Some(fields) = input.as_object_mut() {
                fields.remove("drawing");
                fields.remove("op");
            }
            (
                "run",
                json!({ "command": command, "op": text(args, "op").unwrap_or("execute"), "input": input }),
            )
        }
        other => {
            return Err(refused(
                "not_on_desktop",
                format!("{other} masaüstündeki çizimle kullanılmaz."),
            ));
        }
    };
    let Some(link) = server.desktops.get_mut(handle) else {
        return Err(refused(
            "unknown_drawing",
            "Masaüstü tutamacı artık açık değil.",
        ));
    };
    match link.ask(method, params) {
        Ok(value) if method == "layers" => Ok(answer(json!({ "layers": value }), false)),
        Ok(value) => {
            let failed = method == "run" && value["status"] != "completed";
            Ok(answer(value, failed))
        }
        Err((code, message)) => {
            if code == "desktop_gone" {
                server.desktops.remove(handle);
            }
            Err(refused(&code, message))
        }
    }
}

fn run(server: &mut Server, name: &str, args: &Value) -> Result<Value, Value> {
    if let Some(handle) = text(args, "drawing").filter(|h| server.desktops.contains_key(*h)) {
        let handle = handle.to_owned();
        return on_desktop(server, &handle, name, args);
    }
    match name {
        "desktop.attach" => {
            let path =
                text(args, "path").map_or_else(desktop::default_path, std::path::PathBuf::from);
            let mut link =
                Desktop::connect(path).map_err(|(code, message)| refused(&code, message))?;
            let hello = link
                .ask("hello", json!({}))
                .map_err(|(code, message)| refused(&code, message))?;
            let handle = format!("desktop-{}", Uuid::new_v4());
            server.desktops.insert(handle.clone(), link);
            Ok(answer(
                json!({ "drawing": handle, "desktop": hello }),
                false,
            ))
        }
        "drawing.open" => {
            let path =
                text(args, "path").ok_or_else(|| refused("invalid_input", "path gerekli."))?;
            let session = Session::open(Path::new(path)).map_err(host)?;
            opened(server, session)
        }
        "drawing.new" => {
            let srid = args
                .get("srid")
                .and_then(Value::as_u64)
                .and_then(|s| u32::try_from(s).ok())
                .ok_or_else(|| {
                    refused(
                        "invalid_input",
                        "srid gerekli: projenin koordinat sistemi (ör. 5256).",
                    )
                })?;
            let named = |field: &str, default: &str| -> Result<Value, Value> {
                Ok(json!(text(args, field).unwrap_or(default)))
            };
            let choices = NewDrawing {
                name: text(args, "name").unwrap_or("Adsız proje").to_owned(),
                srid,
                plot_scale: args
                    .get("plotScale")
                    .and_then(Value::as_f64)
                    .unwrap_or(1000.0),
                workspace: serde_json::from_value(named("workspace", "hybrid")?).map_err(|_| {
                    refused(
                        "invalid_input",
                        "workspace: hybrid, cad, gis ya da disaster.",
                    )
                })?,
                drawing_font: serde_json::from_value(named("drawingFont", "barlow")?).map_err(
                    |_| {
                        refused(
                            "invalid_input",
                            "drawingFont bilinen bir yazı tipi değil (barlow, arimo …).",
                        )
                    },
                )?,
            };
            let session = Session::new(&choices).map_err(host)?;
            opened(server, session)
        }
        "drawing.list" => {
            let mut open: Vec<Value> = server
                .desktops
                .iter()
                .map(|(handle, d)| json!({ "drawing": handle, "desktop": true, "link": d.path.display().to_string() }))
                .collect();
            open.extend(server.drawings.iter().map(|(handle, s)| {
                let summary = s.summary();
                json!({
                    "drawing": handle,
                    "name": summary["name"],
                    "path": summary.get("path"),
                    "objects": summary["objects"],
                    "revision": summary["revision"],
                    "dirty": summary["dirty"],
                })
            }));
            Ok(answer(json!({ "drawings": open }), false))
        }
        "drawing.close" => {
            let (s, handle) = session(server, args)?;
            let dirty = s.summary()["dirty"] == true;
            if dirty && args.get("discard") != Some(&json!(true)) {
                return Err(refused(
                    "unsaved",
                    "Çizimde kaydedilmemiş değişiklik var: drawing.save ile kaydedin ya da discard: true ile kapatın.",
                ));
            }
            server.drawings.remove(&handle);
            Ok(answer(json!({ "closed": handle }), false))
        }
        "drawing.save" => {
            let (s, _) = session(server, args)?;
            let saved = s.save(text(args, "path").map(Path::new)).map_err(host)?;
            Ok(answer(saved, false))
        }
        "drawing.summary" => {
            let (s, _) = session(server, args)?;
            Ok(answer(s.summary(), false))
        }
        "drawing.layers" => {
            let (s, _) = session(server, args)?;
            Ok(answer(json!({ "layers": s.layers() }), false))
        }
        "drawing.entities" => {
            let (s, _) = session(server, args)?;
            let kinds = args.get("kinds").and_then(Value::as_array).map(|k| {
                k.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            });
            let bbox = match args.get("bbox").and_then(Value::as_array) {
                Some(b) => {
                    let n: Vec<f64> = b.iter().filter_map(Value::as_f64).collect();
                    let [min_x, min_y, max_x, max_y] = n[..] else {
                        return Err(refused(
                            "invalid_input",
                            "bbox dört sayı olmalı: [min_x, min_y, max_x, max_y].",
                        ));
                    };
                    Some(kentos_contracts::Bounds {
                        min_x,
                        min_y,
                        max_x,
                        max_y,
                    })
                }
                None => None,
            };
            let limit = args
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(PAGE)
                .clamp(1, PAGE_MAX);
            let page = s
                .entities(
                    text(args, "layer").map(str::to_owned),
                    kinds,
                    bbox,
                    text(args, "after").map(str::to_owned),
                    usize::try_from(limit).ok(),
                )
                .map_err(host)?;
            Ok(answer(
                serde_json::to_value(&page).unwrap_or(Value::Null),
                false,
            ))
        }
        "drawing.entity" => {
            let (s, _) = session(server, args)?;
            let uid = text(args, "uid").ok_or_else(|| refused("invalid_input", "uid gerekli."))?;
            Ok(answer(s.entity(uid).map_err(host)?, false))
        }
        "drawing.measure" => {
            let (s, _) = session(server, args)?;
            let uid = text(args, "uid").ok_or_else(|| refused("invalid_input", "uid gerekli."))?;
            let m = s.measure(uid).map_err(host)?;
            Ok(answer(
                serde_json::to_value(&m).unwrap_or(Value::Null),
                false,
            ))
        }
        "project.list" => server
            .account
            .projects(args)
            .map(|page| answer(page, false))
            .map_err(|f| cloud::refusal(&f)),
        "project.open" => {
            let (tenant, project) = (
                text(args, "tenant").ok_or_else(|| refused("invalid_input", "tenant gerekli."))?,
                text(args, "project")
                    .ok_or_else(|| refused("invalid_input", "project gerekli."))?,
            );
            let (bytes, revision) = server
                .account
                .snapshot(tenant, project)
                .map_err(|f| cloud::refusal(&f))?;
            let session = Session::from_bytes(&bytes).map_err(host)?;
            let mut opened = opened(server, session)?;
            opened["structuredContent"]["projectRevision"] = json!(revision);
            opened["content"][0]["text"] =
                json!(serde_json::to_string(&opened["structuredContent"]).unwrap_or_default());
            Ok(opened)
        }
        remote if remote.starts_with("project.") => {
            let Some(c) = server.catalog["commands"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|c| c["id"] == remote)
            else {
                return Err(refused(
                    "unknown_command",
                    format!("{remote} katalogda yok."),
                ));
            };
            let version = c["version"]
                .as_u64()
                .and_then(|v| u32::try_from(v).ok())
                .unwrap_or(1);
            let scope = if c["requires"]
                .as_array()
                .is_some_and(|r| r.iter().any(|x| x == "cloudProject"))
            {
                "project"
            } else {
                "tenant"
            };
            server
                .account
                .command(remote, version, scope, args)
                .map(|output| answer(output, false))
                .map_err(|f| cloud::refusal(&f))
        }
        "drawing.undo" | "drawing.redo" => {
            let (s, _) = session(server, args)?;
            let step = if name == "drawing.undo" {
                s.undo()
            } else {
                s.redo()
            };
            Ok(answer(json!({ "step": step }), false))
        }
        command => {
            let (s, _) = session(server, args)?;
            let op = Op::parse(text(args, "op").unwrap_or("execute")).map_err(host)?;
            let mut input = args.clone();
            if let Some(fields) = input.as_object_mut() {
                fields.remove("drawing");
                fields.remove("op");
            }
            let result = s.run(command, None, op, input).map_err(host)?;
            let failed = result["status"] != "completed";
            Ok(answer(result, failed))
        }
    }
}

/// The catalog, and each open drawing's summary and layers.
pub fn resources(server: &Server) -> Vec<Value> {
    let mut out = vec![json!({
        "uri": "kentos://catalog",
        "name": "catalog",
        "title": "Komut kataloğu",
        "description": "Bütün ürün komutları: sürüm, sunucular, izinler, girdi, çıktı ve plan şemaları, örnekler.",
        "mimeType": "application/json",
    })];
    for (handle, s) in &server.drawings {
        out.push(json!({
            "uri": format!("kentos://drawing/{handle}"),
            "name": format!("drawing-{handle}"),
            "title": s.summary()["name"],
            "description": "Açık çizimin özeti ve katman ağacı.",
            "mimeType": "application/json",
        }));
    }
    out
}

pub fn read(server: &Server, params: &Value) -> Result<Value, Fault> {
    let uri = params
        .get("uri")
        .and_then(Value::as_str)
        .ok_or_else(|| Fault::new(-32602, "Invalid params: uri gerekli."))?;
    let value = if uri == "kentos://catalog" {
        server.catalog.clone()
    } else if let Some(handle) = uri.strip_prefix("kentos://drawing/")
        && let Some(s) = server.drawings.get(handle)
    {
        json!({ "summary": s.summary(), "layers": s.layers() })
    } else {
        return Err(Fault::new(-32602, format!("Resource not found: {uri}")));
    };
    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "application/json",
            "text": serde_json::to_string(&value).unwrap_or_default(),
        }]
    }))
}
