//! Reading an IFC file (docs/adr/8061 §1, §3, §7; plan/ifc-uyum.md IFC-07): the reading face over the engine's STEP
//! parser. Bytes in, plain data out: the file's summary (schema, header, units, map conversion), its objects
//! (GlobalId, class, type, name), the spatial tree, the relations and their inverse index, property and quantity sets
//! on demand, and findings with the line they are on. The engine's types never show on this face (docs/adr/8061 §3.3
//! rule 7): a newer engine changes this module only.
//!
//! Hostile input is the rule here, not the exception. Before the engine sees a record our own bounded scan
//! ([`scan`]) measures it: its length, how deeply its lists nest (the engine parses lists recursively), how many items
//! they hold and the records it refers to; a record past a limit stops the read with its line. The whole read runs
//! behind a panic boundary: an engine panic is an error to report to its makers, never the application's end
//! (CLAUDE.md §14). The host decides the time: `cancel` is asked between records and the read stops when it says so.
//!
//! Damage that leaves the rest readable (a record the engine cannot decode, a reference to nothing, a cycle in the
//! tree or in a placement chain, a GlobalId missing, malformed or said twice) is a finding; the model is still given.
//! Damage that leaves nothing to read (an empty file, no STEP head, an unknown schema, no DATA section) is an error.

#[cfg(feature = "geometry")]
pub mod bodies;
#[cfg(all(test, feature = "geometry"))]
mod bodies_tests;
#[cfg(all(test, feature = "geometry"))]
mod outside_tests;
mod checks;
mod graph;
mod header;
mod scan;
mod sets;
mod units;
#[cfg(test)]
mod reference_tests;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::Arc;

pub use sets::{PropertySet, SetKind, Value, ValueKind};

/// How much a read may take; past a limit the read stops with an error that names it. The defaults are KentOS's
/// (docs/adr/8061 §7, "Bozuk girdi" and "İptal ve bütçe"): generous for real models, small enough that a hostile file
/// cannot exhaust the machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The file's size in bytes.
    pub bytes: u64,
    /// How many records (`#n=…;`) it may hold.
    pub records: usize,
    /// How long one record may be, in bytes.
    pub record_bytes: usize,
    /// How deeply one record's lists may nest. IFC's deepest lists nest three deep (a list of lists of indices).
    pub nesting: usize,
    /// How many items one record's lists may hold together.
    pub items: usize,
    /// How deep the spatial tree and a placement chain may go before the walk calls it a cycle.
    pub depth: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            bytes: 2_000_000_000,
            records: 60_000_000,
            record_bytes: 256 << 20,
            nesting: 32,
            items: 8_000_000,
            depth: 1_000,
        }
    }
}

/// Why nothing could be read.
#[derive(Debug, Clone, PartialEq)]
pub enum ReadError {
    /// The file holds no bytes.
    Empty,
    /// It does not start with `ISO-10303-21;`: not a STEP file.
    NotStep,
    /// `FILE_SCHEMA` names a schema the reader does not know, or none.
    Schema { declared: Option<String> },
    /// Larger than the limit.
    TooLarge { bytes: u64, limit: u64 },
    /// More records than the limit.
    TooManyRecords { limit: usize },
    /// A record past one of the per-record limits, at its line.
    Record { line: usize, what: RecordLimit, limit: usize },
    /// No `DATA;` section, or no record in it.
    NoData,
    /// The host asked the read to stop.
    Cancelled,
    /// The engine panicked: a fault of the engine on this file, to report to its makers. The message is its own.
    Engine { message: String },
}

/// Which per-record limit a record went past.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordLimit {
    Length,
    Nesting,
    Items,
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadError::Empty => write!(f, "Dosya boş: okunacak bir şey yok. Dosyanın tamamının indiğini ya da kaydedildiğini denetleyin."),
            ReadError::NotStep => write!(
                f,
                "Bu bir IFC (ISO 10303-21) dosyası değil: “ISO-10303-21;” ile başlamıyor. ifcXML, ifcZIP ya da başka bir biçimse önce IFC-SPF olarak dışa aktarın."
            ),
            ReadError::Schema { declared: Some(s) } => write!(
                f,
                "Dosyanın şeması “{s}” tanınmıyor; okunabilenler IFC2X3, IFC4, IFC4X1, IFC4X2 ve IFC4X3. Dosyayı bu şemalardan biriyle yeniden dışa aktarın."
            ),
            ReadError::Schema { declared: None } => write!(
                f,
                "Dosyanın başlığında FILE_SCHEMA yok: hangi IFC sürümüyle yazıldığı bilinmiyor. Dosyayı yazan programla yeniden dışa aktarın."
            ),
            ReadError::TooLarge { bytes, limit } => write!(
                f,
                "Dosya {} MB, okuma sınırı {} MB: daha küçük parçalara bölünmüş (disiplin ya da kat başına) bir dışa aktarım deneyin.",
                bytes / 1_000_000,
                limit / 1_000_000
            ),
            ReadError::TooManyRecords { limit } => {
                write!(f, "Dosyada {limit} kayıttan fazlası var; okuma sınırı aşıldı. Modeli parçalara bölerek dışa aktarın.")
            }
            ReadError::Record { line, what, limit } => {
                let what = match what {
                    RecordLimit::Length => format!("{} MB'tan uzun", limit >> 20),
                    RecordLimit::Nesting => format!("listeleri {limit} kattan derin iç içe"),
                    RecordLimit::Items => format!("listelerinde {limit} öğeden fazlası var"),
                };
                write!(f, "{line}. satırdaki kayıt {what}: dosya bozuk ya da olağan dışı. Dosyayı yazan programla yeniden dışa aktarın.")
            }
            ReadError::NoData => write!(
                f,
                "Dosyada DATA bölümü ya da kayıt yok: dosya yarım kalmış olabilir. Dosyanın tamamının kaydedildiğini denetleyin."
            ),
            ReadError::Cancelled => write!(f, "Okuma durduruldu."),
            ReadError::Engine { message } => write!(
                f,
                "IFC okuma motoru bu dosyada beklenmedik biçimde durdu ({message}). Uygulama çalışmayı sürdürüyor; dosyayı bildirin."
            ),
        }
    }
}

impl std::error::Error for ReadError {}

/// The IFC schema family a file is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Ifc2x3,
    Ifc4,
    Ifc4x1,
    Ifc4x2,
    Ifc4x3,
}

/// The file's `HEADER` (ISO 10303-21 §8): as written, its strings decoded.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Header {
    pub description: Vec<String>,
    pub implementation_level: Option<String>,
    pub name: Option<String>,
    pub time_stamp: Option<String>,
    pub author: Vec<String>,
    pub organization: Vec<String>,
    pub preprocessor: Option<String>,
    pub originating_system: Option<String>,
    pub authorization: Option<String>,
}

/// A unit of the project (`IfcProject.UnitsInContext`): its name as written (`METRE`, or a conversion's name such as
/// `foot`), its SI prefix, and what one of it is in SI units.
#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    pub name: String,
    pub prefix: Option<String>,
    pub to_si: f64,
}

/// The project's units for length, area, volume and plane angle; one the file does not give is none.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Units {
    pub length: Option<Unit>,
    pub area: Option<Unit>,
    pub volume: Option<Unit>,
    pub plane_angle: Option<Unit>,
}

/// The map conversion the file declares (`IfcMapConversion`, `IfcMapConversionScaled`) and its target system
/// (`IfcProjectedCRS`): shown, never applied (the viewer does not transform coordinates, CLAUDE.md §5).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Location {
    pub crs: Option<String>,
    pub crs_description: Option<String>,
    pub geodetic_datum: Option<String>,
    pub vertical_datum: Option<String>,
    pub map_projection: Option<String>,
    pub map_zone: Option<String>,
    pub eastings: f64,
    pub northings: f64,
    pub orthogonal_height: f64,
    pub x_axis_abscissa: Option<f64>,
    pub x_axis_ordinate: Option<f64>,
    pub scale: Option<f64>,
    /// `IfcMapConversionScaled`'s factors (IFC 4.3); none for a plain conversion.
    pub factors: Option<[f64; 3]>,
}

/// What the file is: its schema as declared and its family, its header, how many records it holds and of which
/// classes its objects are, its units and its map conversion.
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub schema: String,
    pub family: Family,
    pub header: Header,
    pub records: usize,
    pub classes: BTreeMap<String, usize>,
    pub units: Units,
    pub location: Option<Location>,
}

/// An object of the file (every `IfcObjectDefinition`: the project, the spatial structure, elements, types, groups):
/// its record number, its GlobalId and class as written, its names, and its place in the tree as indices into
/// [`Model::objects`].
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub step: u32,
    pub global_id: String,
    /// The class as the schema spells it (`IfcWall`).
    pub class: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub object_type: Option<String>,
    pub long_name: Option<String>,
    pub tag: Option<String>,
    pub predefined: Option<String>,
    /// The object it is a part of (`IfcRelAggregates`, `IfcRelNests`).
    pub parent: Option<usize>,
    /// The spatial structure that contains it (`IfcRelContainedInSpatialStructure`).
    pub container: Option<usize>,
    /// Its type object (`IfcRelDefinesByType`).
    pub type_object: Option<usize>,
    /// Its parts, then what it contains, in the order the relations list them.
    pub children: Vec<usize>,
    /// Whether it has a shape (`Representation` is set).
    pub has_shape: bool,
}

/// A relationship record (`IfcRel…`): its record number and class, the record on its relating side and those on its
/// related side, by the schema's attribute names (`Relating…`, `Related…`).
#[derive(Debug, Clone, PartialEq)]
pub struct Relation {
    pub step: u32,
    pub class: String,
    pub relating: Option<u32>,
    pub related: Vec<u32>,
}

/// How bad a finding is: an error is damage in the file, a warning something a reader should know.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// Something wrong with the file that did not stop the read: its code, how bad, the record and the line it is on, and
/// what it says (in Turkish, with its remedy).
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub code: &'static str,
    pub severity: Severity,
    pub step: Option<u32>,
    pub line: Option<usize>,
    pub message: String,
}

/// The file read.
#[derive(Debug, Clone)]
pub struct Model {
    pub summary: Summary,
    pub objects: Vec<Object>,
    pub relations: Vec<Relation>,
    /// The tree's roots: objects that are part of nothing and contained in nothing (the project, and anything the
    /// file left out of its tree).
    pub roots: Vec<usize>,
    pub findings: Vec<Finding>,
    by_step: HashMap<u32, usize>,
    inverse: HashMap<u32, Vec<usize>>,
    /// The engine's record index, kept for the sets read on demand; private, so its type stays off the face.
    index: Arc<ifc_lite_core::EntityIndex>,
}

impl Model {
    /// The object with record number `step`.
    pub fn object(&self, step: u32) -> Option<&Object> {
        self.by_step.get(&step).map(|&i| &self.objects[i])
    }

    /// The index of the object with record number `step`.
    pub fn index_of(&self, step: u32) -> Option<usize> {
        self.by_step.get(&step).copied()
    }

    /// The relations that name record `step` on either side (the inverse index): indices into [`Model::relations`].
    pub fn relations_of(&self, step: u32) -> &[usize] {
        self.inverse.get(&step).map_or(&[], Vec::as_slice)
    }
}

/// Reads `bytes` as an IFC file within `limits`, asking `cancel` between records whether to stop.
pub fn read(bytes: &[u8], limits: &Limits, cancel: &dyn Fn() -> bool) -> Result<Model, ReadError> {
    if bytes.is_empty() {
        return Err(ReadError::Empty);
    }
    if bytes.len() as u64 > limits.bytes {
        return Err(ReadError::TooLarge { bytes: bytes.len() as u64, limit: limits.bytes });
    }
    if !header::starts_as_step(bytes) {
        return Err(ReadError::NotStep);
    }
    guarded(|| {
        let (header, schema) = header::read(bytes)?;
        let family = header::family(schema.as_deref()).ok_or(ReadError::Schema { declared: schema.clone() })?;
        let schema = schema.unwrap_or_default();
        let scanned = scan::scan(bytes, limits, cancel)?;
        graph::build(bytes, limits, cancel, header, schema, family, scanned)
    })
}

/// Runs `f` behind the panic boundary: a panic of the engine becomes [`ReadError::Engine`] with its message.
pub(crate) fn guarded<T>(f: impl FnOnce() -> Result<T, ReadError>) -> Result<T, ReadError> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(r) => r,
        Err(payload) => {
            let message = payload
                .downcast_ref::<&str>()
                .map(|s| (*s).to_owned())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "panik".to_owned());
            Err(ReadError::Engine { message })
        }
    }
}
