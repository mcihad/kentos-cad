//! The facts of the IFC file KentOS would write now (docs/adr/8074 §3; plan/ifc-uyum.md IFC-08): the model written in
//! memory as `cad.arch.ifc` writes it and read back by the reading face (`kentos_ifc::read::file_facts`), with what
//! only the source knows: the objects the file is expected to hold (`expected`: persistent id, GlobalId, class;
//! `kentos_ifc::expected`), what the export left out and why it wrote no system (`export`), and where the building is
//! in the drawing (`at`, for a finding that names no object). The standards package's model quality control form
//! (`dijital_proje.form`) reads them: Mimari denetim's Mevzuat findings, the EK-9 table in the drawing and Ruhsat
//! takımı's.
//!
//! Nothing is written to disk. A drawing without walls is no building and has no facts; a file that does not read back
//! is the form's failure, said where it is asked.

use kentos_commands::Entity;
use kentos_domain::{Document, Uuid};
use kentos_ifc::arch::{Why, Written};
use kentos_ifc::building::Building;
use serde_json::{Value, json};

/// The application the in-memory file names: the form reads no version.
const APPLICATION: &str = "KentOS Mimari";

/// The model to write, taken from the drawing on the window's thread; written, read back and measured on any other
/// ([`Planned::facts`]): the long part is off the window.
pub(crate) struct Planned {
    building: Building,
    written: Written,
    at: [f64; 2],
}

/// The model `cad.arch.ifc` would write now; none for a drawing without walls.
pub(crate) fn plan(doc: &Document) -> Option<Planned> {
    let at = doc.entities().find_map(|e| match e {
        Entity::Wall(w) => w.wall.pts.first().map(|p| [p.x, p.y]),
        _ => None,
    })?;
    let (building, written) = kentos_native_application::arch_ifc::building(doc, "", APPLICATION);
    Some(Planned { building, written, at })
}

impl Planned {
    /// The file written in memory, read back, its bodies measured, the source's objects beside them.
    pub(crate) fn facts(self) -> Result<Value, String> {
        let b = &self.building;
        let text = kentos_ifc::building::write(b);
        let name = format!("{}.ifc", b.file);
        let limits = kentos_ifc::read::Limits::default();
        let mut facts = kentos_ifc::read::file_facts(text.as_bytes(), &name, &limits, &|| false).map_err(|e| {
            format!("Yazılan IFC modeli geri okunamadı: {e}. Bu KentOS'un hatasıdır; çizimi bildirin.")
        })?;
        facts["expected"] = kentos_ifc::expected::of(b)
            .iter()
            .map(|e| json!({ "uid": Uuid::from_u128(e.object).to_string(), "globalId": e.global_id, "class": e.class }))
            .collect();
        let left: Vec<Value> =
            self.written.left.iter().map(|l| json!({ "uid": l.id, "what": l.what, "why": why(l.why) })).collect();
        // Written in memory: the delivered file's version is the architect's to check (EK-9/18).
        facts["export"] = json!({ "left": left, "crsWhy": b.crs_why, "memory": true });
        facts["at"] = json!(self.at);
        Ok(facts)
    }
}

/// The standards package's form (`dijital_proje.form`) and its table function.
const PACKAGE: &str = "dijital-proje";
const FORM: &str = "dijital_proje.form";

/// EK-9's form as the drawing's table rows under its header (`dijital_proje.form.tablo`), for the drawing now, the rule
/// asked on this thread: what Ruhsat takımı writes again. A drawing without walls has its questions unanswered.
pub(crate) fn form_rows(doc: &Document) -> Result<Vec<Vec<String>>, String> {
    form_rows_of(plan(doc))
}

/// The rows for a model taken from the drawing ([`plan`]), on any thread.
pub(crate) fn form_rows_of(planned: Option<Planned>) -> Result<Vec<Vec<String>>, String> {
    let facts = planned.map(Planned::facts).transpose()?.unwrap_or(Value::Null);
    let answer = crate::standards::call(PACKAGE, FORM, "tablo", vec![facts])?;
    let rows: Vec<Vec<String>> =
        serde_json::from_value(answer).map_err(|e| format!("Formun satırları okunamadı ({FORM}.tablo): {e}."))?;
    // The package writes the header the drawing knows the form by (`kentos_ifc::quality::FORM_HEADER`).
    match rows.first() {
        Some(h) if kentos_ifc::quality::is_form(h) => Ok(rows),
        _ => Err(format!("Formun başlığı ({FORM}.tablo) KentOS'un tanıdığı başlık değil: paketi güncelleyin.")),
    }
}

/// Why the export left an object out, as its warning says it.
fn why(w: Why) -> &'static str {
    match w {
        Why::NoBody => "kesit modeli gövdesini kuramadı",
        Why::NoWall => "duvarı yazılmadı",
        Why::NoArea => "alanı yok",
        Why::OneWay => "katında hep bir yönde; IFC'nin aks ızgarası iki yönde aks ister",
    }
}
