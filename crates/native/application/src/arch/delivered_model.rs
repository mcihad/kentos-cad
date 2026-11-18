//! The model the drawing last delivered (docs/adr/8116; Dijital Proje Yön. Md. 15: every delivered model has its model
//! quality control form; plan/bim-plan.md K4: the form is answered from the written file): the newest delivery of the
//! drawing's record (docs/adr/8099) whose package carries a BIM model (docs/adr/8075), read from its folder as it went,
//! and how the drawing now differs from it.
//!
//! - **As it went:** the folder's `yayin.json` is the one the record names (its SHA-256); the model, the form's record
//!   and the drawing the package carries are the bytes the list names. Anything else is a folder changed after the
//!   delivery: the model is not read and the reason names the file. A name in the list is a file of the folder itself
//!   (the carried drawing under `kaynaklar/`), never a path out of it.
//! - **Which delivery:** the newest by number whose list names a model in its BIM part; an older or newer package of
//!   sheets alone is passed over, one recorded without its folder too. A folder that is not there or changed stops the
//!   search and is said: the model it holds is not known.
//! - **The answers** that went with the model: the form's record's (`kalite-kontrol-formu.json`).
//! - **The drawing now against it** (`kentos_ifc::compare`, docs/adr/8103): objects by GlobalId added, removed or
//!   changed, against the model the drawing would write now for the same building (the program's building whose EK-2
//!   name the file bears, else the whole project). Every GlobalId the export writes stays the same for the same drawing
//!   (docs/adr/8117); a file written before that (its storeys' GlobalIds not derived ones) is not compared object by
//!   object: its parts' GlobalIds were numbered, and every object would look moved.
//! - **The expectations** EK-9/8, /17 and /21 read (the source's objects and what its export left out): from the drawing
//!   the package carries, else from the drawing now when its model is the delivered one, else none: the form then leaves
//!   those questions to review and says the list was not given.

use std::path::Path;

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{Delivery, FormAnswer, IFC_COMPARE_TOLERANCE};
use kentos_domain::Document;
use serde_json::{Value, json};

use crate::delivery_compare::{DRAWING, MANIFEST};
use crate::ifc_facts::{self, Planned};

/// The application the drawing's own model names; the comparison and the facts read no header.
const APPLICATION: &str = "KentOS Mimari";

/// A delivered model read as it went.
pub struct Delivered {
    /// The delivery's number, its EK-2 revision and its day (YYYY-AA-GG).
    pub number: u32,
    pub revision: String,
    pub at: String,
    /// The model's file name (EK-2's) and the folder it is in.
    pub model: String,
    pub folder: String,
    /// The author's answers that went with it (docs/adr/8077).
    pub answers: Vec<FormAnswer>,
    bytes: Vec<u8>,
    /// The drawing the package carries, when its list names one that reads.
    source: Option<Document>,
}

/// The drawing's newest delivered model.
pub enum Last {
    /// No delivery's package names a model.
    None,
    Found(Box<Delivered>),
    /// A delivery's package could not be read as it went: the delivery's number and why, with what to do.
    Unreadable { number: u32, why: String },
}

/// How the drawing now differs from the delivered model, objects by GlobalId.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Drift {
    /// Only the drawing's model has them.
    pub added: usize,
    /// Only the delivered model has them.
    pub removed: usize,
    pub changed: usize,
}

impl Drift {
    pub fn is_none(&self) -> bool {
        self.added == 0 && self.removed == 0 && self.changed == 0
    }
}

/// Where the source's objects the form compares the file with came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expected {
    /// The drawing the package carries.
    Carried,
    /// The drawing now: its model is the delivered one.
    Now,
    /// None: the drawing changed and the package carries no drawing.
    Missing,
}

/// How the drawing now stands against the delivered model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counted {
    /// Objects by GlobalId added, removed and changed.
    Drift(Drift),
    /// The drawing writes no model now (no walls).
    NoModel,
    /// The delivered file was written before KentOS kept every GlobalId (docs/adr/8117): not compared.
    Older,
}

impl Counted {
    /// Whether the drawing's model is the delivered one.
    pub fn same(&self) -> bool {
        matches!(self, Counted::Drift(d) if d.is_none())
    }
}

/// The delivered model read for the form.
pub struct Read {
    /// The facts the form reads (`dijital_proje.form`), from the delivered file.
    pub facts: Value,
    /// The drawing now against it.
    pub counted: Counted,
    pub expected: Expected,
}

/// The newest delivery whose package names a model, read from its folder.
pub fn last(doc: &Document) -> Last {
    let mut list: Vec<&Delivery> = doc.deliveries().iter().collect();
    list.sort_by_key(|d| std::cmp::Reverse(d.number));
    for d in list {
        let Some(path) = d.package.path.as_deref() else {
            continue;
        };
        match model_of(d, Path::new(path)) {
            Ok(Some(found)) => return Last::Found(Box::new(found)),
            Ok(None) => {}
            Err(why) => return Last::Unreadable { number: d.number, why },
        }
    }
    Last::None
}

impl Delivered {
    /// The drawing now (`doc`) against the delivered model.
    pub fn counted(&self, doc: &Document) -> Result<Counted, String> {
        let older = self.opened()?;
        let now = planned_for(doc, &self.model);
        self.count(&older, now.as_ref())
    }

    /// The facts the form reads from the delivered file, and the drawing now (`doc`) against it.
    pub fn read(&self, doc: &Document) -> Result<Read, String> {
        let older = self.opened()?;
        let now = planned_for(doc, &self.model);
        let counted = self.count(&older, now.as_ref())?;
        if let Some(p) = self.source.as_ref().and_then(|s| planned_for(s, &self.model)) {
            return Ok(Read { facts: p.facts_of(&self.bytes, false)?, counted, expected: Expected::Carried });
        }
        if let Some(p) = now.as_ref().filter(|_| counted.same()) {
            return Ok(Read { facts: p.facts_of(&self.bytes, false)?, counted, expected: Expected::Now });
        }
        // Neither: the file's own facts alone. No expectations and no export record, so the form leaves EK-9/8, /17
        // and /21 to review rather than passing them on an empty list.
        let limits = kentos_ifc::read::Limits::default();
        let mut facts = kentos_ifc::read::file_facts(&self.bytes, &self.model, &limits, &|| false)
            .map_err(|e| format!("{}. teslimin modeli (“{}”) okunamadı: {e}.", self.number, self.model))?;
        if let Some(p) = &now {
            facts["at"] = json!(p.at());
        }
        Ok(Read { facts, counted, expected: Expected::Missing })
    }

    /// The delivered file read.
    fn opened(&self) -> Result<kentos_ifc::read::Model, String> {
        kentos_ifc::read::read(&self.bytes, &kentos_ifc::read::Limits::default(), &|| false)
            .map_err(|e| format!("{}. teslimin modeli (“{}”) okunamadı: {e}.", self.number, self.model))
    }

    /// The drawing's model (`now`; none: no walls) against the delivered one (`older`).
    fn count(&self, older: &kentos_ifc::read::Model, now: Option<&Planned>) -> Result<Counted, String> {
        let Some(now) = now else {
            return Ok(Counted::NoModel);
        };
        // A storey's GlobalId is a derived one since docs/adr/8117: a file whose storeys have none was numbered.
        let storeys: Vec<&kentos_ifc::read::Object> =
            older.objects.iter().filter(|o| o.class.eq_ignore_ascii_case("IfcBuildingStorey")).collect();
        let derived = |o: &&kentos_ifc::read::Object| {
            kentos_ifc::step::unguid(&o.global_id).is_some_and(kentos_ifc::step::is_derived)
        };
        if !storeys.is_empty() && !storeys.iter().any(derived) {
            return Ok(Counted::Older);
        }
        compared(older, &self.bytes, now).map(Counted::Drift)
    }
}

/// `d`'s package in `folder` as it went: its model, when its list names one; none for a package of sheets alone.
fn model_of(d: &Delivery, folder: &Path) -> Result<Option<Delivered>, String> {
    let n = d.number;
    let list_bytes = std::fs::read(folder.join(MANIFEST)).map_err(|_| {
        format!(
            "{n}. teslimin klasöründe {MANIFEST} yok (“{}”): kayıttaki paketin klasörünü yerine koyun.",
            folder.display()
        )
    })?;
    if sha256_hex(&list_bytes) != d.package.manifest {
        return Err(format!(
            "“{}” klasöründeki {MANIFEST} {n}. teslimin paketi değil (SHA-256 farklı): klasör teslimden sonra değişmiş ya \
             da başka bir yayın. Kayıttaki paketin klasörünü yerine koyun.",
            folder.display()
        ));
    }
    let list: Value = serde_json::from_slice(&list_bytes)
        .map_err(|_| format!("{n}. teslimin {MANIFEST} dosyası bir KentOS yayınının listesi olarak okunamadı."))?;
    let Some(model) = list["bim"]["model"].as_str() else {
        return Ok(None);
    };
    let model_sha = listed(&list["files"], "name", model)
        .ok_or_else(|| format!("{n}. teslimin listesi modeli (“{model}”) özetiyle saymıyor: paket eksik yazılmış."))?;
    let bytes = checked(folder, model, &model_sha, n)?;
    // The form's record: the answers that went with the model.
    let answers = match (list["bim"]["form"]["name"].as_str(), list["bim"]["form"]["sha256"].as_str()) {
        (Some(name), Some(sha)) => {
            let record: Value = serde_json::from_slice(&checked(folder, name, sha, n)?)
                .map_err(|_| format!("{n}. teslimin form kaydı (“{name}”) okunamadı."))?;
            serde_json::from_value(record["answers"].clone()).unwrap_or_default()
        }
        _ => Vec::new(),
    };
    // The drawing the package carries, when its list names it (Paftaları toplu yayınla's sources).
    let source = match listed(&list["resources"]["files"], "path", DRAWING) {
        Some(sha) => {
            let b = checked(folder, DRAWING, &sha, n)?;
            crate::file_import_block::read_drawing(&b).ok().and_then(|s| Document::from_snapshot_v2(s).ok())
        }
        None => None,
    };
    Ok(Some(Delivered {
        number: n,
        revision: d.revision.clone(),
        at: d.at.clone(),
        model: model.to_owned(),
        folder: folder.display().to_string(),
        answers,
        bytes,
        source,
    }))
}

/// The SHA-256 the list's entries (`key` naming them) give `name`.
fn listed(entries: &Value, key: &str, name: &str) -> Option<String> {
    entries
        .as_array()?
        .iter()
        .find(|e| e[key].as_str() == Some(name))
        .and_then(|e| e["sha256"].as_str())
        .map(str::to_owned)
}

/// The bytes of `name` in `folder` when they are the list's (`sha`); a name that leaves the folder is refused.
fn checked(folder: &Path, name: &str, sha: &str, n: u32) -> Result<Vec<u8>, String> {
    let inside = !name.is_empty()
        && !name.starts_with(['/', '\\'])
        && !name.contains('\\')
        && !name.contains(':')
        && name.split('/').all(|part| !part.is_empty() && part != "." && part != "..");
    if !inside {
        return Err(format!("{n}. teslimin listesindeki “{name}” klasörün bir dosyası değil: paket bozuk."));
    }
    let bytes = std::fs::read(folder.join(name))
        .map_err(|_| format!("{n}. teslimin “{name}” dosyası klasörde yok: teslim edilen dosyayı yerine koyun."))?;
    if !sha256_hex(&bytes).eq_ignore_ascii_case(sha) {
        return Err(format!(
            "{n}. teslimin “{name}” dosyası teslimden sonra değişmiş (SHA-256 listedekinden farklı): teslim edilen \
             dosyayı yerine koyun."
        ));
    }
    Ok(bytes)
}

/// The model `doc` would write now for the delivered file `name`: the program's building whose EK-2 name the file
/// bears, else the whole project's. None for a drawing without walls.
fn planned_for(doc: &Document, name: &str) -> Option<Planned> {
    let whole = ifc_facts::plan(doc, None, "", APPLICATION)?;
    if whole.file_name() == name {
        return Some(whole);
    }
    let buildings: Vec<String> = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| a.program.buildings.iter().map(|b| b.id.clone()).collect())
        .unwrap_or_default();
    for id in &buildings {
        if let Some(p) = ifc_facts::plan(doc, Some(id), "", APPLICATION)
            && p.file_name() == name
        {
            return Some(p);
        }
    }
    Some(whole)
}

/// The drawing's model against the delivered file (`older`, read from `delivered`).
fn compared(older: &kentos_ifc::read::Model, delivered: &[u8], now: &Planned) -> Result<Drift, String> {
    let text = now.text();
    let newer = kentos_ifc::read::read(text.as_bytes(), &kentos_ifc::read::Limits::default(), &|| false)
        .map_err(|e| format!("Çizimin modeli geri okunamadı: {e}. Bu KentOS'un hatasıdır; çizimi bildirin."))?;
    let c = kentos_ifc::compare::compare(older, delivered, &newer, text.as_bytes(), IFC_COMPARE_TOLERANCE, &|| false)
        .map_err(|e| format!("Teslim edilen model çizimle karşılaştırılamadı: {e}."))?;
    Ok(Drift { added: c.added.len(), removed: c.removed.len(), changed: c.changed.len() })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An empty drawing.
    fn empty() -> Document {
        let text = r#"{"format":"kentos.document","version":1,"name":"t","settings":{"srid":5256,"lengthDecimals":3,
            "areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000},"origin":{"x":0,"y":0},
            "layers":[{"id":"a","name":"Çizim","type":"layer","visible":true,"locked":false,"expanded":true,
            "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
            "activeLayer":"a","entities":[],"styles":{"items":[],"categories":[]}}"#;
        Document::from_snapshot(kentos_commands::DocumentSnapshotV1::from_json(text).expect("reads")).expect("opens")
    }

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("kentos-teslim-modeli-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a folder");
        dir
    }

    /// A drawing whose record has one delivery of `folder`'s list as it is now.
    fn delivered(folder: &Path) -> Document {
        let mut doc = empty();
        let manifest = std::fs::read(folder.join(MANIFEST)).map(|b| sha256_hex(&b)).unwrap_or_else(|_| "0".repeat(64));
        let input = json!({"change": {"kind": "add", "revision": "000", "package": {"manifest": manifest, "files": 1,
            "path": folder.display().to_string()}, "to": "Örnek Belediyesi", "at": "2026-11-18"}});
        let answer = crate::registry::run(
            &mut crate::ExecutionContext::new(&mut doc),
            "cad.delivery.edit",
            kentos_commands::CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
        doc
    }

    fn write_list(folder: &Path, list: &Value) {
        std::fs::write(folder.join(MANIFEST), serde_json::to_vec(list).expect("json")).expect("writes");
    }

    fn unreadable(doc: &Document) -> String {
        match last(doc) {
            Last::Unreadable { number, why } => format!("{number}: {why}"),
            Last::Found(_) => panic!("read"),
            Last::None => panic!("no model"),
        }
    }

    #[test]
    fn a_package_of_sheets_alone_carries_no_model() {
        let dir = temp("paftalar");
        write_list(&dir, &json!({"format": "kentos.publish/1", "files": [{"name": "001.pdf", "sha256": "ab"}]}));
        assert!(matches!(last(&delivered(&dir)), Last::None));
        assert!(matches!(last(&empty()), Last::None), "no delivery");
    }

    #[test]
    fn a_folder_changed_after_the_delivery_is_said_and_not_read() {
        let dir = temp("degisti");
        let model = b"ISO-10303-21;";
        std::fs::write(dir.join("model.ifc"), model).expect("writes");
        let list = json!({"format": "kentos.publish/1", "files": [{"name": "model.ifc", "sha256": sha256_hex(model)}],
            "bim": {"model": "model.ifc"}});
        write_list(&dir, &list);
        let doc = delivered(&dir);
        // The list rewritten after the delivery.
        write_list(&dir, &json!({"format": "kentos.publish/1", "files": [], "bim": {"model": "model.ifc"}}));
        let why = unreadable(&doc);
        assert!(why.starts_with("1: ") && why.contains("teslimden sonra değişmiş"), "{why}");
        // The model's bytes changed under the same list.
        write_list(&dir, &list);
        std::fs::write(dir.join("model.ifc"), b"ISO-10303-21; degisti").expect("writes");
        let why = unreadable(&doc);
        assert!(why.contains("“model.ifc” dosyası teslimden sonra değişmiş"), "{why}");
        // The model gone.
        std::fs::remove_file(dir.join("model.ifc")).expect("removes");
        assert!(unreadable(&doc).contains("klasörde yok"));
        // The folder gone.
        let _ = std::fs::remove_dir_all(&dir);
        assert!(unreadable(&doc).contains("yayin.json yok"));
    }

    #[test]
    fn a_name_that_leaves_the_folder_is_refused() {
        let dir = temp("kacis");
        let list = json!({"format": "kentos.publish/1", "files": [{"name": "../model.ifc", "sha256": "ab"}],
            "bim": {"model": "../model.ifc"}});
        write_list(&dir, &list);
        let why = unreadable(&delivered(&dir));
        assert!(why.contains("klasörün bir dosyası değil"), "{why}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
