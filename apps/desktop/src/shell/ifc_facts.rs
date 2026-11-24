//! EK-9's form as the drawing's table rows (docs/adr/8074 §3): the facts of the IFC file KentOS would write now
//! (`kentos_native_application::ifc_facts`, written in memory and read back) answered by the standards package
//! (`dijital_proje.form.tablo`), for the EK-9 table in the drawing and Ruhsat takımı's. Mimari denetim asks the same
//! facts (arch_audit/standards.rs).

use kentos_domain::Document;
pub use kentos_native_application::ifc_facts::Planned;
use serde_json::Value;

/// The application the in-memory file names: the form reads no version.
const APPLICATION: &str = "KentOS Mimari";

/// The standards package's form (`dijital_proje.form`) and its table function.
const PACKAGE: &str = "dijital-proje";
const FORM: &str = "dijital_proje.form";

/// The model `cad.arch.ifc` would write now (the whole project's); none for a drawing without walls.
pub fn plan(doc: &Document) -> Option<Planned> {
    kentos_native_application::ifc_facts::plan(doc, None, "", APPLICATION)
}

/// EK-9's form as the drawing's table rows under its header (`dijital_proje.form.tablo`), for the drawing now, the rule
/// asked on this thread: what Ruhsat takımı writes again. A drawing without walls has its questions unanswered.
pub fn form_rows(doc: &Document) -> Result<Vec<Vec<String>>, String> {
    form_rows_of(plan(doc), doc.form_answers().to_vec())
}

/// The rows for a model taken from the drawing ([`plan`]), on any thread, with the author's answers the drawing keeps
/// (docs/adr/8077: the questions KentOS leaves to them take their Evet or Hayır and note).
pub(crate) fn form_rows_of(
    planned: Option<Planned>,
    answers: Vec<kentos_commands::FormAnswer>,
) -> Result<Vec<Vec<String>>, String> {
    let facts = planned.map(Planned::facts).transpose()?.unwrap_or(Value::Null);
    let answers = serde_json::to_value(answers).unwrap_or(Value::Null);
    let answer = crate::standards::call(PACKAGE, FORM, "tablo", vec![facts, answers])?;
    let rows: Vec<Vec<String>> =
        serde_json::from_value(answer).map_err(|e| format!("Formun satırları okunamadı ({FORM}.tablo): {e}."))?;
    // The package writes the header the drawing knows the form by (`kentos_ifc::quality::FORM_HEADER`).
    match rows.first() {
        Some(h) if kentos_ifc::quality::is_form(h) => Ok(rows),
        _ => Err(format!("Formun başlığı ({FORM}.tablo) KentOS'un tanıdığı başlık değil: paketi güncelleyin.")),
    }
}
