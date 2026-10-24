//! A record as the rules see it: the layer's fields as the expression language's typed fields, the
//! object's attributes as their values, and what the object is.

use std::collections::BTreeMap;

use kentos_contracts::{LayerField, LayerFieldKind};
use kentos_expression::{
    Expr, FieldDef, FieldSource, FieldType, Measured, Schema, Scope, Unit, Variable,
};

/// An object's attributes: a field's name to its canonical text; empty is no attribute.
pub type Attrs = BTreeMap<String, String>;

/// What the rules see of the object beside its attributes (`$tür`, `$katman`, `$kimlik`,
/// `$uzunluk`, `$alan`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Object {
    /// Its kind as the language names it (`Kapalı alan`, `Nokta` …).
    pub kind: String,
    /// Its layer's name.
    pub layer: String,
    pub id: f64,
    pub label: Option<String>,
    /// Its length, area and anchor in metres (`$uzunluk`, `$alan`, `$merkez_x` …): the schema's unit
    /// turns them into the drawing's (docs/adr/0402 §5).
    pub measured: Measured,
    /// Its corners (`$köşe_sayısı`).
    pub vertices: Option<f64>,
    /// Its persistent id: what a related item finds the objects referring to it by (docs/adr/0404 §4).
    pub uid: Option<String>,
    /// Its attachments, counted (docs/adr/0407 §1); none where the record's attachments are not
    /// judged (a new record, the document's following of a write).
    pub attachments: Option<Attached>,
    /// Its geometry: what a spatial list asks the other layer's objects about (docs/adr/0439 §5);
    /// none for a record without geometry.
    pub shape: Option<std::sync::Arc<kentos_geometry_core::entity::Shape>>,
    /// The objects a submission adds referring to the record, by [`added_key`](crate::added_key) (the
    /// other layer and field; docs/adr/0444 §1): counted with those already there, a link taken off -1.
    pub added: std::collections::BTreeMap<String, i32>,
}

/// How many attachments an object holds: all of them and its photos.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Attached {
    pub all: u32,
    pub photos: u32,
}

impl Attached {
    /// The counts of `list`.
    pub fn of(list: &[kentos_contracts::Attachment]) -> Attached {
        let photos = list
            .iter()
            .filter(|a| a.kind() == kentos_contracts::AttachmentKind::Photo)
            .count();
        Attached {
            all: u32::try_from(list.len()).unwrap_or(u32::MAX),
            photos: u32::try_from(photos).unwrap_or(u32::MAX),
        }
    }

    /// What an item accepting `accept` counts.
    pub fn counted(&self, accept: kentos_contracts::AttachmentAccept) -> u32 {
        match accept {
            kentos_contracts::AttachmentAccept::Any => self.all,
            kentos_contracts::AttachmentAccept::Photo => self.photos,
            kentos_contracts::AttachmentAccept::Document => self.all - self.photos,
        }
    }
}

/// The expression schema of a layer's fields: each a user-defined field of its kind, so a rule reads
/// a yes/no field as true or false and a number field as a number; the project's `@` values.
pub fn schema_of(fields: &[LayerField], variables: &[Variable]) -> Schema {
    schema_in(fields, variables, Unit::METRE)
}

/// [`schema_of`] whose geometry values speak `unit` (the project's drawing unit: docs/adr/0402 §5).
pub fn schema_in(fields: &[LayerField], variables: &[Variable], unit: Unit) -> Schema {
    Schema {
        fields: fields
            .iter()
            .map(|f| FieldDef {
                name: f.name.clone(),
                ty: type_of(f.kind),
                source: FieldSource::User,
                description: f.alias.clone().unwrap_or_default(),
            })
            .collect(),
        variables: variables.to_vec(),
        world: false,
        unit,
    }
}

/// The expression language's type of a field kind.
fn type_of(kind: LayerFieldKind) -> FieldType {
    match kind {
        LayerFieldKind::Integer | LayerFieldKind::Decimal => FieldType::Number,
        LayerFieldKind::Boolean => FieldType::Bool,
        // ISO text, which compares in time order: a date and time as a date (docs/adr/0412 §3).
        LayerFieldKind::Date | LayerFieldKind::DateTime => FieldType::Date,
        LayerFieldKind::Time | LayerFieldKind::Text => FieldType::Text,
        // docs/adr/0629 §3: read as text, never a float.
        LayerFieldKind::BigInt
        | LayerFieldKind::Instant
        | LayerFieldKind::Uuid
        | LayerFieldKind::Json
        | LayerFieldKind::Binary => FieldType::Text,
    }
}

/// One record seen by one expression.
pub(crate) struct RuleScope<'a> {
    pub expr: &'a Expr,
    pub attrs: &'a Attrs,
    pub object: &'a Object,
}

impl Scope for RuleScope<'_> {
    fn field(&self, i: usize) -> Option<&str> {
        let name = self.expr.fields.get(i)?;
        self.attrs.get(name).map(String::as_str)
    }
    fn measured(&self) -> Measured {
        self.object.measured
    }
    fn vertices(&self) -> Option<f64> {
        self.object.vertices
    }
    fn kind(&self) -> &str {
        &self.object.kind
    }
    fn layer(&self) -> &str {
        &self.object.layer
    }
    fn label(&self) -> Option<&str> {
        self.object.label.as_deref()
    }
    fn index(&self) -> f64 {
        1.0
    }
    fn id(&self) -> f64 {
        self.object.id
    }
    fn scale(&self) -> Option<f64> {
        None
    }
}

/// Whether `expr` is true for the record (an empty answer is not).
pub(crate) fn holds(expr: &Expr, attrs: &Attrs, object: &Object) -> bool {
    kentos_expression::value::truthy(&evaluate(expr, attrs, object))
}

/// `expr`'s value for the record.
pub(crate) fn evaluate(
    expr: &Expr,
    attrs: &Attrs,
    object: &Object,
) -> kentos_expression::Value<'static> {
    let scope = RuleScope {
        expr,
        attrs,
        object,
    };
    expr.evaluate(&scope).into_owned()
}

/// `expr`'s value for the record as a field's text (docs/adr/0402 §3): a yes/no `true` or `false`, a
/// number and a text as the language writes them (`None`: empty).
pub(crate) fn field_text_of(expr: &Expr, attrs: &Attrs, object: &Object) -> Option<String> {
    match evaluate(expr, attrs, object) {
        kentos_expression::Value::Bool(b) => Some(if b { "true" } else { "false" }.to_owned()),
        v if kentos_expression::value::is_empty(&v) => None,
        v => Some(kentos_expression::value::to_text(&v).into_owned()),
    }
}

/// `expr`'s value for the record as text, as the language writes a value (`None`: empty).
pub(crate) fn text_of(expr: &Expr, attrs: &Attrs, object: &Object) -> Option<String> {
    let v = evaluate(expr, attrs, object);
    if kentos_expression::value::is_empty(&v) {
        return None;
    }
    Some(kentos_expression::value::to_text(&v).into_owned())
}
