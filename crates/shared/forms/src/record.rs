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
    /// The related records a form's `ilişkili_toplam` reads (docs/adr/0761): the form gives them; none
    /// elsewhere, where the call is empty.
    pub related: Option<crate::relatives::RelatedRecords>,
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

/// The schema a form's own rules compile against: [`schema_in`] where `ilişkili_toplam` reads the
/// record's related records (docs/adr/0761).
pub fn form_schema_in(fields: &[LayerField], variables: &[Variable], unit: Unit) -> Schema {
    Schema {
        relations: true,
        ..schema_in(fields, variables, unit)
    }
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
        relations: false,
        unit,
        // A rule (a condition, a check, a default) offers no extension's function: it would be judged before the
        // answer came. A calculation does ([`calc_schema`]).
        externals: Vec::new(),
    }
}

/// The schema a field's calculation compiles against (docs/adr/8057 §8): `base` with the host's extensions'
/// functions known. While typing a calculation reads the answers the host has (one not had yet is empty, asked in
/// the background); on saving the host gives answers it waits for ([`answering_with`]).
pub fn calc_schema(base: &Schema) -> Schema {
    Schema {
        externals: kentos_expression::external::host_defs(),
        ..base.clone()
    }
}

thread_local! {
    /// What answers the extensions' functions instead of the host's own, while [`answering_with`] runs.
    static ANSWERING: std::cell::RefCell<Option<kentos_expression::external::HostCalls>> =
        const { std::cell::RefCell::new(None) };
}

/// Runs `f` with `calls` answering the extensions' functions a calculation calls: a host's answers it waits for, on
/// saving a record (the value goes into the drawing, it is not left empty because the answer had not come yet).
pub fn answering_with<R>(
    calls: kentos_expression::external::HostCalls,
    f: impl FnOnce() -> R,
) -> R {
    let kept = ANSWERING.with(|a| a.replace(Some(calls)));
    let out = f();
    ANSWERING.with(|a| *a.borrow_mut() = kept);
    out
}

/// What answers `expr`'s extensions' functions now: [`answering_with`]'s, else the host's that never waits.
fn calls_for(expr: &Expr) -> Option<kentos_expression::external::HostCalls> {
    if expr.externals().is_empty() {
        return None;
    }
    ANSWERING
        .with(|a| a.borrow().clone())
        .or_else(kentos_expression::external::host_calls)
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
    /// What answers its extensions' functions (a calculation's, docs/adr/8057 §8).
    pub calls: Option<kentos_expression::external::HostCalls>,
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
    fn world(&self) -> Option<&dyn kentos_expression::world::WorldCalls> {
        self.object
            .related
            .as_ref()
            .map(|_| self as &dyn kentos_expression::world::WorldCalls)
    }
    fn externals(&self) -> Option<&dyn kentos_expression::external::ExternalCalls> {
        self.calls
            .as_deref()
            .map(|c| c as &dyn kentos_expression::external::ExternalCalls)
    }
}

/// `ilişkili_toplam` answered over the record's related records (docs/adr/0761).
impl kentos_expression::world::WorldCalls for RuleScope<'_> {
    fn call(
        &self,
        k: usize,
        _id: f64,
        _own: &[kentos_expression::scalar::V],
        out: &mut String,
    ) -> kentos_expression::scalar::R<'static> {
        let (Some(c), Some(related)) = (self.expr.world.get(k), &self.object.related) else {
            return kentos_expression::scalar::R::V(kentos_expression::scalar::V::Null);
        };
        related.value(c, self.object.uid.as_deref(), out)
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
        calls: calls_for(expr),
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use kentos_contracts::{LayerField, LayerFieldKind};
    use kentos_expression::compile_with;
    use kentos_expression::external::{Arg, ExternalCalls, ExternalDef};

    use super::*;

    const NAME: &str = "ext.tr.ornek.imar-denetim.ada-parsel";

    /// The host as a drawing asks it: no answer yet.
    struct Coming;

    impl ExternalCalls for Coming {
        fn call(&self, _: &str, rows: &[Vec<Arg>]) -> Vec<Arg> {
            vec![Arg::Null; rows.len()]
        }
    }

    /// The host as a save asks it: it waits for the extension's answer.
    struct Waiting;

    impl ExternalCalls for Waiting {
        fn call(&self, name: &str, rows: &[Vec<Arg>]) -> Vec<Arg> {
            assert_eq!(name, NAME);
            rows.iter()
                .map(|r| match (&r[0], &r[1]) {
                    (Arg::Text(a), Arg::Text(p)) => Arg::Text(format!("{a}/{p}")),
                    _ => Arg::Null,
                })
                .collect()
        }
    }

    #[test]
    fn a_calculation_calls_an_extensions_function_and_waits_for_it_on_saving() {
        // docs/adr/8057 §8: while typing the answer not had yet is empty; saving waits for it; a rule does not know
        // the function (it would be judged before the answer came).
        kentos_expression::external::set_host(
            vec![ExternalDef {
                name: NAME.into(),
                description: "Ada/parsel".into(),
            }],
            Arc::new(Coming),
        );
        let fields = [
            LayerField::new("ada", LayerFieldKind::Text),
            LayerField::new("parsel", LayerFieldKind::Text),
        ];
        let rules = schema_of(&fields, &[]);
        let source = format!("{NAME}(ada, parsel)");
        let e = compile_with(&source, &calc_schema(&rules)).expect("a calculation knows it");
        assert!(compile_with(&source, &rules).is_err(), "a rule does not");
        let attrs: Attrs = [("ada", "101"), ("parsel", "2")]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        let object = Object::default();
        assert_eq!(
            field_text_of(&e, &attrs, &object),
            None,
            "typing: not answered yet"
        );
        let saved = answering_with(Arc::new(Waiting), || field_text_of(&e, &attrs, &object));
        assert_eq!(saved.as_deref(), Some("101/2"), "saving: waited for");
        assert_eq!(
            field_text_of(&e, &attrs, &object),
            None,
            "the host's own again after"
        );
    }
}
