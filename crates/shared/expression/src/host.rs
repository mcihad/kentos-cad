//! What a host tells the engine about its objects (docs/adr/0100 §3): the
//! fields an expression can name, with their types and where they come from
//! (`Schema`), and the objects' values a batch at a time (`Objects`).
//!
//! A name in an expression is one of three things:
//! - a built-in value, written with `$`: the object's geometry values
//!   (`Geometry`), kind, layer, label, id, position;
//! - a user-defined field the schema declares, with its type: its values
//!   come as typed columns, a number field as numbers, never text to parse;
//! - otherwise one of today's text attributes (the object's key-value
//!   `attrs`), empty where the object has none.
//!
//! The schema itself (who defines fields, where they are stored) is not this
//! crate's; it only reads what the host gives it (TODOS.md `DOM-09`–`DOM-11`).

use crate::Expr;
use crate::exec::{Slot, Source};
use crate::js::text::fold_turkish;
use crate::program::Load;
use crate::rows::{As, Column, column};
use crate::value::Value;
use crate::world::{Session, WithWorld, World};

/// A field's type, as the schema declares it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldType {
    Text,
    Number,
    /// True/false.
    Bool,
    /// A calendar date, as ISO text (YYYY-AA-GG): it compares and prints as
    /// that text. Date arithmetic is a later function set.
    Date,
}

/// Where a field's values come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldSource {
    /// The object's own values: geometry, kind, layer, label, id (`$` names).
    Builtin,
    /// Today's key-value text attributes.
    Attribute,
    /// A field the user defined for the layer, with a type.
    User,
}

/// A field the host knows: its name as written, type, source and one line of help.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldDef {
    pub name: String,
    pub ty: FieldType,
    pub source: FieldSource,
    pub description: String,
}

/// A `@` value (docs/adr/0214 §2.3): one of the project's own variables or
/// a built-in one (the project's name, the date …), the same for every
/// object of an evaluation.
#[derive(Clone, Debug, PartialEq)]
pub struct Variable {
    /// As written after `@`; looked for with Turkish letters and case aside.
    pub name: String,
    pub value: Value<'static>,
    /// One line for the builder.
    pub description: String,
}

/// The fields of the objects an expression runs on: user-defined typed
/// fields, and the attributes the host has seen (for completion; an
/// attribute it has not listed is still read, as text); and the `@`
/// variables, the project's first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Schema {
    pub fields: Vec<FieldDef>,
    pub variables: Vec<Variable>,
    /// Whether the functions that look at other layers can be used here
    /// (İşlemler and the builder over them, docs/adr/0214 §1); the builder
    /// refuses and hides them elsewhere.
    pub world: bool,
}

impl Schema {
    /// A `@` variable by name (`@Proje_Adı` is `@proje_adi`): the first of that name.
    pub fn variable(&self, name: &str) -> Option<&Variable> {
        let key = fold_turkish(name);
        self.variables.iter().find(|v| fold_turkish(&v.name) == key)
    }

    /// The field a name in an expression means (names are matched exactly,
    /// as attribute keys are).
    pub fn find(&self, name: &str) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// What a field of an expression is: the schema's typed field, else a text attribute.
    pub fn resolve(&self, name: &str) -> FieldRef {
        match self.find(name) {
            Some(f) if f.source == FieldSource::User => FieldRef {
                ty: f.ty,
                source: FieldSource::User,
            },
            _ => FieldRef {
                ty: FieldType::Text,
                source: FieldSource::Attribute,
            },
        }
    }
}

/// A field of a compiled expression, resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FieldRef {
    pub ty: FieldType,
    pub source: FieldSource,
}

/// A geometry value of an object, computed only when an expression reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Geometry {
    /// Length of a path, perimeter of an area (holes included).
    Length,
    Area,
    /// Y (east) and X (north) of the object's anchor (where its label sits).
    AnchorY,
    AnchorX,
    /// Y and X of the area's centroid (holes removed); the anchor for other objects.
    CentroidY,
    CentroidX,
    /// The bounding box, Y east and X north.
    MinY,
    MaxY,
    MinX,
    MaxX,
    /// Extent east (Y) and north (X).
    Width,
    Height,
    /// Corners of a path or area (holes included).
    Vertices,
}

/// The object's own values other than geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Builtin {
    /// The kind's label: “Kapalı alan”, “Çizgi” … (text).
    Kind,
    /// The layer's name (text).
    Layer,
    /// The label shown in the drawing (text; none: empty).
    Label,
    /// The object's number.
    Id,
    /// Denominator of the plot scale while drawing a symbol (none elsewhere).
    Scale,
}

/// A host's objects, as the engine asks for them: a column of a batch at a
/// time (`start .. start + slot.len()`), only for what the expression reads.
pub trait Objects<'a> {
    /// How many objects.
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Field `name`'s values: text for attributes and text fields (ISO text
    /// for dates), numbers for number fields, true/false for true/false
    /// fields; empty where an object has no value.
    fn field(&self, name: &str, ty: FieldType, start: usize, slot: Slot<'_, 'a>);

    /// A geometry value (`geometry::Shapes` computes them from shapes).
    fn geometry(&self, what: Geometry, start: usize, slot: Slot<'_, 'a>);

    /// Kind, layer, label, id or plot scale.
    fn builtin(&self, what: Builtin, start: usize, slot: Slot<'_, 'a>);
}

/// A host's objects as the engine reads them.
struct ObjectsSource<'e, 'o, 'a> {
    expr: &'e Expr,
    objects: &'o (dyn Objects<'a> + 'o),
}

impl<'a> Source<'a> for ObjectsSource<'_, '_, 'a> {
    fn fill(&self, load: Load, start: usize, mut slot: Slot<'_, 'a>) {
        let o = self.objects;
        let geometry = |what, slot| o.geometry(what, start, slot);
        match load {
            Load::Field(i) => {
                let i = i as usize;
                match (self.expr.fields.get(i), self.expr.types.get(i)) {
                    (Some(name), Some(r)) => o.field(name, r.ty, start, slot),
                    _ => (0..slot.len()).for_each(|k| slot.text(k, None)),
                }
            }
            Load::Area => geometry(Geometry::Area, slot),
            Load::Length => geometry(Geometry::Length, slot),
            Load::Y => geometry(Geometry::AnchorY, slot),
            Load::X => geometry(Geometry::AnchorX, slot),
            Load::CentroidY => geometry(Geometry::CentroidY, slot),
            Load::CentroidX => geometry(Geometry::CentroidX, slot),
            Load::MinY => geometry(Geometry::MinY, slot),
            Load::MaxY => geometry(Geometry::MaxY, slot),
            Load::MinX => geometry(Geometry::MinX, slot),
            Load::MaxX => geometry(Geometry::MaxX, slot),
            Load::Width => geometry(Geometry::Width, slot),
            Load::Height => geometry(Geometry::Height, slot),
            Load::Vertices => geometry(Geometry::Vertices, slot),
            Load::Kind => o.builtin(Builtin::Kind, start, slot),
            Load::Layer => o.builtin(Builtin::Layer, start, slot),
            Load::Label => o.builtin(Builtin::Label, start, slot),
            Load::Id => o.builtin(Builtin::Id, start, slot),
            Load::Scale => o.builtin(Builtin::Scale, start, slot),
            Load::Index => slot.numbers(|k| (true, (start + k + 1) as f64)),
        }
    }
}

impl Expr {
    /// The value for every object of a host, each as `want` asks, in one
    /// column (the layout `rows::Column` documents): a batch of objects at a
    /// time, only what the expression reads asked of the host.
    pub fn evaluate_objects<'a, 'o>(
        &self,
        objects: &'o (dyn Objects<'a> + 'o),
        want: As,
    ) -> Column {
        let source = ObjectsSource {
            expr: self,
            objects,
        };
        column(self, &source, objects.len(), want)
    }

    /// `evaluate_objects` with the world its calls to other objects are
    /// answered from (docs/adr/0214 §3).
    pub fn evaluate_objects_in<'a, 'o>(
        &self,
        objects: &'o (dyn Objects<'a> + 'o),
        want: As,
        world: Option<&World<'_>>,
    ) -> Column {
        let source = ObjectsSource {
            expr: self,
            objects,
        };
        match world.filter(|_| self.looks_around()) {
            Some(world) => {
                let session = Session::new(self, world);
                let with = WithWorld {
                    inner: &source,
                    world: &session,
                };
                column(self, &with, objects.len(), want)
            }
            None => column(self, &source, objects.len(), want),
        }
    }
}

/// A host's objects as a source for an expression (a world's layer's, `LayerObjects`).
pub fn objects_source<'e, 'o, 'a>(
    expr: &'e Expr,
    objects: &'o (dyn Objects<'a> + 'o),
) -> impl Source<'a> + use<'e, 'o, 'a> {
    ObjectsSource { expr, objects }
}

/// `objects_source` for objects held by value (a small view of the host's).
pub fn objects_source_of<'e, 'a: 'e, O: Objects<'a> + 'e>(
    expr: &'e Expr,
    objects: O,
) -> impl Source<'a> + 'e {
    OwnedSource {
        expr,
        objects,
        _texts: std::marker::PhantomData,
    }
}

struct OwnedSource<'e, 'a, O: Objects<'a>> {
    expr: &'e Expr,
    objects: O,
    _texts: std::marker::PhantomData<&'a ()>,
}

impl<'a, O: Objects<'a>> Source<'a> for OwnedSource<'_, 'a, O> {
    fn fill(&self, load: Load, start: usize, slot: Slot<'_, 'a>) {
        ObjectsSource {
            expr: self.expr,
            objects: &self.objects,
        }
        .fill(load, start, slot);
    }
}
