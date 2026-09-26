//! The shared product command cases (fixtures/commands/v1, docs/adr/0022,
//! 0027, 0029, 0032) run against the desktop's handlers over the native document. The web
//! runs the same files against its own handlers
//! (apps/web/src/product/fixtures.test.ts); the format is in
//! fixtures/commands/README.md.
//!
//! JSON carries no NaN or ±∞: a step's `nonFinite` puts them into the typed
//! input after it is read, at the paths the errors name.

use std::collections::HashMap;
use std::path::PathBuf;

use kentos_domain::contracts::{
    ArcCreate, CAD_ARC_CREATE, CAD_CIRCLE_CREATE, CAD_ENTITIES_DELETE, CAD_LINE_CREATE,
    CAD_POINT_CREATE, CAD_POLYGON_CREATE, CAD_POLYLINE_CREATE, CircleCreate, DocumentSnapshotV1,
    EntitiesDelete, LineCreate, PointCreate, PolygonCreate, PolylineCreate,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_native_application::{
    DESKTOP_COMMANDS, ExecutionContext, arc, circle, delete, line, point, polygon, polyline,
};
use serde_json::{Value, json};

type Outcome<T> = Result<T, String>;

#[derive(Default)]
struct State {
    /// Revisions taken by `captureRevision`, as the text commands use.
    revisions: HashMap<String, String>,
    /// Persistent ids taken by `captureUid`.
    uids: HashMap<String, Uuid>,
}

const STEP_KEYS: &[&str] = &[
    "op",
    "input",
    "nonFinite",
    "result",
    "returns",
    "as",
    "id",
    "expect",
    "note",
];

/// JSON equality with numbers compared as numbers (the file's `1` is the document's `1.0`).
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

fn expect_same(got: &Value, want: &Value, what: &str, at: &str) -> Outcome<()> {
    if same(got, want) {
        Ok(())
    } else {
        Err(format!("{at}: {what}: beklenen {want}, bulunan {got}"))
    }
}

/// `value` with its `$…` placeholders filled in: `$current` is the
/// document's revision now, `$name` one `captureRevision` took; `$uid:name`,
/// anywhere in a text (an id list, a message), the persistent id
/// `captureUid` took, lowercase with hyphens.
fn fill(value: &Value, doc: &Document, state: &State, at: &str) -> Outcome<Value> {
    Ok(match value {
        Value::String(text) if text.contains("$uid:") => Value::String(with_uids(text, state, at)?),
        Value::String(text) if text.starts_with('$') => {
            let name = &text[1..];
            let revision = if name == "current" {
                doc.revision().to_string()
            } else {
                state
                    .revisions
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("{at}: “{name}” sürümü alınmadı"))?
            };
            Value::String(revision)
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|v| fill(v, doc, state, at))
                .collect::<Outcome<_>>()?,
        ),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(k, v)| Ok((k.clone(), fill(v, doc, state, at)?)))
                .collect::<Outcome<_>>()?,
        ),
        other => other.clone(),
    })
}

/// `text` with every `$uid:name` replaced by the persistent id `captureUid` took as `name`.
fn with_uids(text: &str, state: &State, at: &str) -> Outcome<String> {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("$uid:") {
        out.push_str(&rest[..start]);
        let after = &rest[start + "$uid:".len()..];
        let end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
            .unwrap_or(after.len());
        let name = &after[..end];
        let uid = state
            .uids
            .get(name)
            .ok_or_else(|| format!("{at}: “{name}” kimliği alınmadı"))?;
        out.push_str(&uid.to_string());
        rest = &after[end..];
    }
    out.push_str(rest);
    Ok(out)
}

/// The number of a point list at `rest` (`pts[1].y`, after any ring prefix).
fn point_number<'a>(
    pts: &'a mut [kentos_domain::contracts::Vec2],
    rest: &str,
) -> Option<&'a mut f64> {
    let (i, axis) = rest.strip_prefix("pts[")?.split_once("].")?;
    let p = pts.get_mut(i.parse::<usize>().ok()?)?;
    match axis {
        "x" => Some(&mut p.x),
        "y" => Some(&mut p.y),
        _ => None,
    }
}

/// The bulge at `rest` (`bulges[3]`, after any ring prefix).
fn bulge_number<'a>(bulges: &'a mut Option<Vec<f64>>, rest: &str) -> Option<&'a mut f64> {
    let i = rest.strip_prefix("bulges[")?.strip_suffix(']')?;
    bulges.as_mut()?.get_mut(i.parse::<usize>().ok()?)
}

/// A command's typed input, with the numbers `nonFinite` may name.
trait Input {
    /// The number at an error path; `None` when the input has no such place.
    fn number(&mut self, path: &str) -> Option<&mut f64>;
}

impl Input for PolygonCreate {
    /// `pts[1].y`, `bulges[3]`, `holes[0].pts[2].x`, `holes[1].bulges[0]`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let (pts, bulges, rest) = match path.strip_prefix("holes[") {
            Some(rest) => {
                let (h, rest) = rest.split_once("].")?;
                let ring = self.holes.as_mut()?.get_mut(h.parse::<usize>().ok()?)?;
                (&mut ring.pts, &mut ring.bulges, rest)
            }
            None => (&mut self.pts, &mut self.bulges, path),
        };
        if rest.starts_with("pts[") {
            point_number(pts, rest)
        } else {
            bulge_number(bulges, rest)
        }
    }
}

impl Input for LineCreate {
    /// `a.x`, `b.y`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let (end, axis) = path.split_once('.')?;
        let p = match end {
            "a" => &mut self.a,
            "b" => &mut self.b,
            _ => return None,
        };
        match axis {
            "x" => Some(&mut p.x),
            "y" => Some(&mut p.y),
            _ => None,
        }
    }
}

impl Input for PolylineCreate {
    /// `pts[1].y`, `bulges[0]`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path.starts_with("pts[") {
            point_number(&mut self.pts, path)
        } else {
            bulge_number(&mut self.bulges, path)
        }
    }
}

impl Input for EntitiesDelete {
    /// No number: the input is ids.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

/// A coordinate of a named point: `c.x`, `p.y`.
fn coordinate<'a>(
    p: &'a mut kentos_domain::contracts::Vec2,
    name: &str,
    path: &str,
) -> Option<&'a mut f64> {
    match path.strip_prefix(name)?.strip_prefix('.')? {
        "x" => Some(&mut p.x),
        "y" => Some(&mut p.y),
        _ => None,
    }
}

impl Input for PointCreate {
    /// `p.x`, `p.y`, `z` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "z" => self.z.as_mut(),
            _ => coordinate(&mut self.p, "p", path),
        }
    }
}

impl Input for CircleCreate {
    /// `c.x`, `c.y`, `r`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "r" => Some(&mut self.r),
            _ => coordinate(&mut self.c, "c", path),
        }
    }
}

impl Input for ArcCreate {
    /// `c.x`, `c.y`, `r`, `a0`, `a1`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "r" => Some(&mut self.r),
            "a0" => Some(&mut self.a0),
            "a1" => Some(&mut self.a1),
            _ => coordinate(&mut self.c, "c", path),
        }
    }
}

/// Puts NaN or ±∞ into the typed input at the paths the step's `nonFinite` names.
fn put_non_finite(input: &mut impl Input, step: &Value, at: &str) -> Outcome<()> {
    let Some(table) = step.get("nonFinite") else {
        return Ok(());
    };
    let table = table
        .as_object()
        .ok_or_else(|| format!("{at}: nonFinite bir nesne olmalı"))?;
    for (path, value) in table {
        let number = match value.as_str() {
            Some("NaN") => f64::NAN,
            Some("Infinity") => f64::INFINITY,
            Some("-Infinity") => f64::NEG_INFINITY,
            _ => {
                return Err(format!(
                    "{at}: {path}: NaN, Infinity ya da -Infinity olmalı"
                ));
            }
        };
        *input
            .number(path)
            .ok_or_else(|| format!("{at}: girdide böyle bir yer yok: {path}"))? = number;
    }
    Ok(())
}

/// Runs `op` of the fixture's command on the document; its whole result as the wire carries it.
fn run_op(
    command: &str,
    op: &str,
    doc: &mut Document,
    input: Value,
    step: &Value,
    at: &str,
) -> Outcome<Value> {
    // The step's input, read as the command's type, with its non-finite numbers put in.
    macro_rules! run {
        ($module:ident, $input:ty) => {{
            let mut input: $input =
                serde_json::from_value(input).map_err(|e| format!("{at}: girdi okunamadı: {e}"))?;
            put_non_finite(&mut input, step, at)?;
            match op {
                "validate" => {
                    serde_json::to_value($module::validate(&ExecutionContext::new(doc), &input))
                }
                "plan" => serde_json::to_value($module::plan(&ExecutionContext::new(doc), &input)),
                _ => serde_json::to_value($module::execute(&mut ExecutionContext::new(doc), input)),
            }
        }};
    }
    match command {
        CAD_POLYGON_CREATE => run!(polygon, PolygonCreate),
        CAD_LINE_CREATE => run!(line, LineCreate),
        CAD_POLYLINE_CREATE => run!(polyline, PolylineCreate),
        CAD_ENTITIES_DELETE => run!(delete, EntitiesDelete),
        CAD_POINT_CREATE => run!(point, PointCreate),
        CAD_CIRCLE_CREATE => run!(circle, CircleCreate),
        CAD_ARC_CREATE => run!(arc, ArcCreate),
        other => return Err(format!("{at}: {other} için koşucu yok")),
    }
    .map_err(|e| format!("{at}: sonuç yazılamadı: {e}"))
}

/// Runs one command step and compares its whole result.
fn run_command(
    command: &str,
    doc: &mut Document,
    state: &State,
    step: &Value,
    at: &str,
) -> Outcome<()> {
    let op = step["op"].as_str().unwrap_or("?");
    let input = fill(&step["input"], doc, state, at)?;
    let got = run_op(command, op, doc, input, step, at)?;
    let Some(want) = step.get("result") else {
        return Err(format!("{at}: komut adımında “result” yok"));
    };
    let mut want = want.clone();
    // “$uid”: the persistent id of the object the command wrote, lowercase with hyphens.
    if want.pointer("/output/uid") == Some(&json!("$uid")) {
        let text = got
            .pointer("/output/uid")
            .and_then(Value::as_str)
            .unwrap_or("");
        let slot = got.pointer("/output/id").and_then(Value::as_u64);
        let uid =
            Uuid::parse_str(text).map_err(|_| format!("{at}: kimlik UUID değil: “{text}”"))?;
        let own = slot
            .and_then(|s| u32::try_from(s).ok())
            .and_then(|s| doc.uid(Slot(s)));
        if uid.to_string() != text || own != Some(uid) {
            return Err(format!(
                "{at}: output.uid {text}, yuvadaki nesnenin kimliği {own:?}"
            ));
        }
        if let Some(field) = want.pointer_mut("/output/uid") {
            *field = Value::String(text.to_owned());
        }
    }
    expect_same(&got, &fill(&want, doc, state, at)?, "sonuç", at)
}

fn run_step(
    command: &str,
    doc: &mut Document,
    state: &mut State,
    step: &Value,
    at: &str,
) -> Outcome<()> {
    if let Some(fields) = step.as_object() {
        // A misspelt field would otherwise pass unchecked.
        if let Some(key) = fields.keys().find(|k| !STEP_KEYS.contains(&k.as_str())) {
            return Err(format!("{at}: bilinmeyen alan “{key}”"));
        }
    }
    let before = doc.revision();
    match step["op"].as_str().unwrap_or("?") {
        "validate" | "plan" | "execute" => run_command(command, doc, state, step, at)?,
        op @ ("undo" | "redo") => {
            let label = if op == "undo" { doc.undo() } else { doc.redo() };
            if let Some(want) = step.get("returns") {
                expect_same(&json!(label), want, "dönen değer", at)?;
            }
        }
        "captureRevision" => {
            let name = step["as"].as_str().ok_or(format!("{at}: “as” yok"))?;
            state
                .revisions
                .insert(name.to_owned(), doc.revision().to_string());
        }
        "captureUid" => {
            let name = step["as"].as_str().ok_or(format!("{at}: “as” yok"))?;
            let id = step["id"].as_u64().and_then(|n| u32::try_from(n).ok());
            let uid = id
                .and_then(|id| doc.uid(Slot(id)))
                .ok_or(format!("{at}: {id:?} nesnesi yok"))?;
            state.uids.insert(name.to_owned(), uid);
        }
        other => return Err(format!("{at}: bilinmeyen işlem “{other}”")),
    }
    check(doc, state, step.get("expect"), before, at)
}

fn check(
    doc: &Document,
    state: &State,
    expect: Option<&Value>,
    before: u64,
    at: &str,
) -> Outcome<()> {
    let Some(expect) = expect else {
        return Ok(());
    };
    let expect = expect
        .as_object()
        .ok_or(format!("{at}: expect bir nesne olmalı"))?;
    let empty = serde_json::Map::new();
    for (key, want) in expect {
        match key.as_str() {
            "ids" => {
                let ids: Vec<u32> = doc.entities().map(|e| e.base().id).collect();
                expect_same(&json!(ids), want, "nesneler", at)?;
            }
            "entities" => {
                for (id, entity) in want.as_object().unwrap_or(&empty) {
                    let slot = id
                        .parse::<u32>()
                        .map(Slot)
                        .map_err(|_| format!("{at}: {id}"))?;
                    let got = doc
                        .get(slot)
                        .map(serde_json::to_value)
                        .transpose()
                        .map_err(|e| format!("{at}: {e}"))?
                        .unwrap_or(Value::Null);
                    expect_same(&got, entity, &format!("nesne {id}"), at)?;
                }
            }
            "canUndo" => expect_same(&json!(doc.can_undo()), want, "canUndo", at)?,
            "canRedo" => expect_same(&json!(doc.can_redo()), want, "canRedo", at)?,
            "dirty" => expect_same(&json!(doc.is_dirty()), want, "dirty", at)?,
            "revision" => {
                let got = if doc.revision() == before {
                    "same"
                } else {
                    "changed"
                };
                expect_same(&json!(got), want, "sürüm", at)?;
            }
            "uids" => {
                for (id, name) in want.as_object().unwrap_or(&empty) {
                    let slot = id
                        .parse::<u32>()
                        .map(Slot)
                        .map_err(|_| format!("{at}: {id}"))?;
                    let uid = doc
                        .uid(slot)
                        .ok_or(format!("{at}: {id} nesnesinin kalıcı kimliği yok"))?;
                    match name.as_str() {
                        Some("new") if state.uids.values().any(|u| *u == uid) => {
                            return Err(format!("{at}: {id} nesnesinin kimliği yeni değil"));
                        }
                        Some("new") => {}
                        Some(name) if state.uids.get(name) == Some(&uid) => {}
                        Some(name) => {
                            return Err(format!(
                                "{at}: {id} nesnesinin kimliği {uid}, “{name}” {:?}",
                                state.uids.get(name)
                            ));
                        }
                        None => return Err(format!("{at}: kimlik adı metin olmalı")),
                    }
                }
            }
            // A misspelt expectation would otherwise pass unchecked.
            other => return Err(format!("{at}: bilinmeyen beklenti “{other}”")),
        }
    }
    Ok(())
}

fn run_case(command: &str, setup: &Value, case: &Value, at: &str) -> Outcome<()> {
    let snapshot = DocumentSnapshotV1::from_json(&setup.to_string())
        .map_err(|e| format!("{at}: kurulum: {e}"))?;
    let mut doc = Document::from_snapshot(snapshot).map_err(|e| format!("{at}: kurulum: {e}"))?;
    if doc.is_dirty() || doc.can_undo() {
        return Err(format!("{at}: kurulumdan sonra belge temiz değil"));
    }
    let steps = case["steps"].as_array().ok_or(format!("{at}: adım yok"))?;
    if steps.is_empty() {
        return Err(format!("{at}: adım yok"));
    }
    let mut state = State::default();
    for (i, step) in steps.iter().enumerate() {
        let op = step["op"].as_str().unwrap_or("?");
        run_step(
            command,
            &mut doc,
            &mut state,
            step,
            &format!("{at} › {} {op}", i + 1),
        )?;
    }
    Ok(())
}

#[test]
fn every_command_case_matches_the_desktop_handlers() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/commands/v1");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("fixtures/commands/v1")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "fixtures/commands/v1: no files");
    let mut problems = Vec::new();
    let mut cases = 0;
    let mut commands = Vec::new();
    for path in &files {
        let file = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let fixture: Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
                .expect("JSON");
        assert_eq!(fixture["format"], "kentos.command-cases", "{file}");
        assert_eq!(fixture["version"], 1, "{file}");
        let command = (
            fixture["command"].as_str().unwrap_or(""),
            fixture["commandVersion"].as_u64().unwrap_or(0),
        );
        assert!(
            DESKTOP_COMMANDS
                .iter()
                .any(|(id, v)| (*id, u64::from(*v)) == command),
            "{file}: {command:?} is not a desktop command"
        );
        commands.push(command.0.to_owned());
        let listed = fixture["cases"].as_array().expect("cases");
        assert!(listed.len() >= 20, "{file}: {} cases", listed.len());
        for case in listed {
            cases += 1;
            let name = case["name"].as_str().unwrap_or("?");
            let setup = case.get("setup").unwrap_or(&fixture["setup"]);
            if let Err(problem) = run_case(command.0, setup, case, &format!("{file} › {name}")) {
                problems.push(problem);
            }
        }
    }
    // Every command the desktop runs has its cases.
    for (id, _) in DESKTOP_COMMANDS {
        assert!(
            commands.iter().any(|c| c == id),
            "{id}: no file in fixtures/commands/v1"
        );
    }
    assert!(
        problems.is_empty(),
        "{} of {cases} cases failed:\n{}",
        problems.len(),
        problems.join("\n")
    );
}
