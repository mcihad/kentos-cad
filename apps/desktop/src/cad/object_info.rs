//! Eylemler and Bilgi kartı (docs/adr/0225 §4–§6; the web's
//! `app/objectActions.ts`): a layer's actions filled for an object and
//! opened, and its information card's title, rows and text for an object.
//! Templates are split and filled by the expression core
//! (`kentos_native_application::info_templates`); `{$enlem}` and `{$boylam}`
//! are the object's label point moved from the project's system into WGS 84
//! the way Koordinat dönüştür moves it.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use kentos_commands::{Entity, InfoCard, LayerAction};
use kentos_domain::Slot;
use kentos_expression::template::{TargetKind, target_kind};
use kentos_geometry_core::crs::{Datum, System, transform_in};
use kentos_native_application::expression_unit::Unit;
use kentos_native_application::geometry::shape;
use kentos_native_application::info_templates::{CompiledTemplate, compile_template};
use kentos_native_application::layer_filter::kind_label;
use kentos_project::systems;

use crate::app::App;

/// A source line and whether it waits to be written, by the drawing's revision and the slot.
type SourceLines = HashMap<(String, u32), Option<(String, bool)>>;

thread_local! {
    /// The objects' source lines by the drawing's revision and the slot: the base is read once per change, not at
    /// every frame the card shows.
    static SOURCES: RefCell<SourceLines> = RefCell::new(HashMap::new());
    /// The templates compiled, by their text (the web's `compileTemplate`
    /// keeps them too): the card and the actions are filled at every frame
    /// they show.
    static COMPILED: RefCell<HashMap<String, Result<Rc<CompiledTemplate>, String>>> =
        RefCell::new(HashMap::new());
}

/// `text` as a template in `unit` (the drawing's readings', `App::reading_unit`),
/// compiled once per text and unit.
pub(crate) fn compiled(text: &str, unit: Unit) -> Result<Rc<CompiledTemplate>, String> {
    COMPILED.with(|cache| {
        let mut cache = cache.borrow_mut();
        let key = format!("{}\u{0}{text}", unit.factor());
        if let Some(known) = cache.get(&key) {
            return known.clone();
        }
        let made = compile_template(text, unit).map(Rc::new);
        if cache.len() > 500 {
            cache.clear();
        }
        cache.insert(key, made.clone());
        made
    })
}

/// An action for one object: its name, its filled target (none when its
/// template is broken: `error` says why) and what it is.
#[derive(Clone, Debug)]
pub struct ObjectAction {
    pub name: String,
    pub target: Option<String>,
    pub error: Option<String>,
    pub kind: TargetKind,
}

/// An object's card: its title, its rows (an attribute's name and value), its
/// text, whether it shows attachments and actions, and why a template of it
/// is broken when one is (what it would show is shown without it).
#[derive(Clone, Debug, Default)]
pub struct ObjectCard {
    pub title: String,
    pub rows: Vec<(String, String)>,
    pub text: Option<String>,
    pub attachments: bool,
    pub actions: bool,
    pub error: Option<String>,
    /// Where the object stands with its bound table (docs/adr/0651 §1): the line and whether it waits to be written.
    pub source: Option<(String, bool)>,
}

impl App {
    /// The object's label point in WGS 84 as (latitude, longitude); none
    /// without the project's own system or a point.
    pub(crate) fn place_of(&self, e: &Entity) -> Option<(f64, f64)> {
        let doc = self.document.as_ref()?;
        let settings = doc.model.settings();
        let own = systems::own(settings)?.system?;
        let at = kentos_geometry_core::entity::entity_anchor(&shape(e))?;
        let wgs84 = System::Geographic {
            datum: Datum::Wgs84,
        };
        let moved = transform_in(&own, &wgs84, at, &systems::choices(settings)).ok()?;
        Some((moved.point.y, moved.point.x))
    }

    /// A template's text for one object: its geometry values from its shape,
    /// its place when the template asks for it.
    fn filled(&self, t: &CompiledTemplate, e: &Entity) -> String {
        let place = if t.needs_place() {
            self.place_of(e)
        } else {
            None
        };
        let shapes = if t.reads_geometry() {
            vec![shape(e)]
        } else {
            Vec::new()
        };
        let name = |id: &str| self.layer_name(id);
        t.texts(&[e], &name, &|i| shapes.get(i), &|_| place)
            .into_iter()
            .next()
            .unwrap_or_default()
    }

    /// The object's layer's actions (or `actions`, a window's draft), each filled for it.
    pub(crate) fn object_actions(
        &self,
        e: &Entity,
        actions: Option<&[LayerAction]>,
    ) -> Vec<ObjectAction> {
        let own;
        let list = match actions {
            Some(list) => list,
            None => {
                own = self
                    .document
                    .as_ref()
                    .and_then(|d| d.model.layers().get(&e.base().layer_id))
                    .map(|n| n.actions.clone())
                    .unwrap_or_default();
                &own
            }
        };
        let unit = self.reading_unit();
        list.iter()
            .map(|a| match compiled(&a.target, unit) {
                Err(why) => ObjectAction {
                    name: a.name.clone(),
                    target: None,
                    error: Some(why),
                    kind: TargetKind::Refused,
                },
                Ok(t) => {
                    let target = self.filled(&t, e).trim().to_owned();
                    ObjectAction {
                        name: a.name.clone(),
                        kind: target_kind(&target),
                        target: Some(target),
                        error: None,
                    }
                }
            })
            .collect()
    }

    /// The actions of the object in `slot`.
    pub(crate) fn slot_actions(&self, slot: Slot) -> Vec<ObjectAction> {
        self.document
            .as_ref()
            .and_then(|d| d.model.get(slot))
            .map(|e| self.object_actions(e, None))
            .unwrap_or_default()
    }

    /// Opens an object's action: an address and a file in the system's
    /// opener; a target that is no http, https or mailto address and no
    /// file's path is not opened, and neither is a file that is no document
    /// or picture (an executable); both are said.
    pub(crate) fn run_action(&mut self, action: &ObjectAction) {
        if let Some(why) = &action.error {
            self.warn(format!("“{}” eyleminin hedefi: {why}", action.name));
            return;
        }
        let target = action.target.clone().unwrap_or_default();
        match action.kind {
            TargetKind::Address => {
                if let Err(e) = crate::sheet_pdf::open_with_viewer(std::path::Path::new(&target)) {
                    self.warn(format!("“{}” açılamadı: {e}.", action.name));
                }
            }
            TargetKind::File => self.open_document(&action.name, &target),
            TargetKind::Refused => self.warn(format!(
                "“{}” eyleminin hedefi açılmadı: “{target}” http, https ya da mailto adresi ya da dosya yolu değil.",
                action.name
            )),
        }
    }

    /// A linked document or picture opened in the system's opener; a file of
    /// another kind is not (docs/adr/0225 §4: no program is run).
    pub(crate) fn open_document(&mut self, name: &str, file: &str) {
        let folder = self
            .document
            .as_ref()
            .and_then(|d| d.path.as_deref())
            .and_then(std::path::Path::parent);
        let path = crate::pictures::resolve(file, folder);
        if !opens(&path) {
            self.warn(format!(
                "“{name}” açılmadı: “{}” bir belge ya da resim değil (PDF, TXT, CSV, JPEG, PNG, TIFF, DOC, XLS, ODT, ODS, DWG, DXF açılır).",
                path.display()
            ));
            return;
        }
        if !path.exists() {
            self.warn(format!("“{name}” bulunamadı: {}.", path.display()));
            return;
        }
        if let Err(e) = crate::sheet_pdf::open_with_viewer(&path) {
            self.warn(format!("“{name}” açılamadı: {e}."));
        }
    }

    /// The card of an object: `card` (a window's draft, or the layer's: [`App::slot_card`]),
    /// or the default card (its kind and label, every attribute).
    pub(crate) fn object_card(&self, e: &Entity, card: Option<&InfoCard>) -> ObjectCard {
        let mut error = None;
        let unit = self.reading_unit();
        let mut text = |source: Option<&String>, what: &str| -> Option<String> {
            let source = source?;
            match compiled(source, unit) {
                Ok(t) => Some(self.filled(&t, e)),
                Err(why) => {
                    error.get_or_insert_with(|| format!("{what}: {why}"));
                    None
                }
            }
        };
        let base = e.base();
        let kind = kind_label(e);
        let title =
            text(card.and_then(|c| c.title.as_ref()), "Kartın başlığı").unwrap_or_else(|| {
                match &base.label {
                    Some(label) => format!("{kind} {label}"),
                    None => kind.to_owned(),
                }
            });
        let body = text(card.and_then(|c| c.text.as_ref()), "Kartın metni");
        let fields: Vec<String> = match card.map(|c| &c.fields) {
            Some(f) if !f.is_empty() => f.clone(),
            _ => base.attrs.keys().cloned().collect(),
        };
        let rows = fields
            .into_iter()
            .map(|f| {
                let v = base.attrs.get(&f).cloned().unwrap_or_default();
                (f, v)
            })
            .collect();
        ObjectCard {
            title,
            rows,
            text: body,
            attachments: card.is_none_or(|c| c.attachments),
            actions: card.is_none_or(|c| c.actions),
            error,
            source: None,
        }
    }

    /// The card of the object in `slot`, by its layer's card, with its source line when its layer is bound.
    pub(crate) fn slot_card(&self, slot: Slot) -> Option<ObjectCard> {
        let doc = self.document.as_ref()?;
        let e = doc.model.get(slot)?;
        let card = doc
            .model
            .layers()
            .get(&e.base().layer_id)
            .and_then(|n| n.card.as_ref());
        let mut out = self.object_card(e, card);
        out.source = source_line(&doc.model, slot);
        Some(out)
    }
}

/// The object's source line (docs/adr/0651 §1), cached by the drawing's revision.
fn source_line(doc: &kentos_domain::Document, slot: Slot) -> Option<(String, bool)> {
    let key = (doc.revision().to_string(), slot.0);
    SOURCES.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(known) = cache.get(&key) {
            return known.clone();
        }
        let line =
            kentos_native_application::sources::object_source(doc, slot).map(|o| source_text(&o));
        if cache.len() > 200 {
            cache.clear();
        }
        cache.insert(key, line.clone());
        line
    })
}

/// “Kaynak: parsel · ada 101 — Kaynağa yazılmadı”, and whether it waits to be written.
pub(crate) fn source_text(o: &kentos_commands::ObjectSource) -> (String, bool) {
    use kentos_commands::ObjectSourceState as S;
    let mut text = format!("Kaynak: {}", o.table);
    if let (Some(k), Some(v)) = (&o.key, &o.value) {
        text.push_str(&format!(" · {k} {v}"));
    }
    let (state, waits) = match o.state {
        S::Matched => ("kaynakla aynı", false),
        S::Edited => ("değişti, kaynağa yazılmadı", true),
        S::Added => ("yeni, kaynakta yok", true),
        S::Unknown => ("katman değişti; bu nesneyi Kaynak durumu söyler", false),
    };
    text.push_str(" — ");
    text.push_str(state);
    if !o.writable {
        text.push_str(" (salt okunur)");
    }
    (text, waits)
}

/// The files an action or an attachment opens: documents and pictures by their extension.
pub(crate) fn opens(path: &std::path::Path) -> bool {
    const KINDS: [&str; 16] = [
        "pdf", "txt", "csv", "jpg", "jpeg", "png", "tif", "tiff", "doc", "docx", "xls", "xlsx",
        "odt", "ods", "dwg", "dxf",
    ];
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| KINDS.contains(&e.to_ascii_lowercase().as_str()))
}
