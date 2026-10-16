//! A record as the rules see it: the layer's fields as the expression language's typed fields, the
//! object's attributes as their values, and what the object is.

use std::collections::BTreeMap;

use kentos_contracts::{LayerField, LayerFieldKind};
use kentos_expression::{
    Expr, FieldDef, FieldSource, FieldType, Measured, Schema, Scope, Variable,
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
    pub measured: Measured,
}

/// The expression schema of a layer's fields: each a user-defined field of its kind, so a rule reads
/// a yes/no field as true or false and a number field as a number; the project's `@` values.
pub fn schema_of(fields: &[LayerField], variables: &[Variable]) -> Schema {
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
        ..Default::default()
    }
}

/// The expression language's type of a field kind.
fn type_of(kind: LayerFieldKind) -> FieldType {
    match kind {
        LayerFieldKind::Integer | LayerFieldKind::Decimal => FieldType::Number,
        LayerFieldKind::Boolean => FieldType::Bool,
        LayerFieldKind::Date => FieldType::Date,
        LayerFieldKind::Text => FieldType::Text,
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
        None
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
    let scope = RuleScope {
        expr,
        attrs,
        object,
    };
    kentos_expression::value::truthy(&expr.evaluate(&scope))
}
