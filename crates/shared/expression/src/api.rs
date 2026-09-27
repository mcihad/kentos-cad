//! The expression builder's services in the browser's call table
//! (docs/adr/0100 §5): `kentos-geometry-wasm` looks a name up here after the
//! geometry core's and the style core's tables. Arguments and results are
//! JSON; positions are UTF-16 units from 0. `fixtures/expression/v2/builder.json`
//! pins every operation's answers on both platforms.

use kentos_geometry_core::api::Op;
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::{json_struct, op};

use crate::editor::{self, Arg, Diagnostic, Help, Item, Section, ValueItem};
use crate::{FieldDef, FieldSource, FieldType, Schema, Value};

/// A field as the page describes it: `{ name, type, source?, description? }`.
struct FieldJson {
    name: String,
    ty: String,
    source: Option<String>,
    description: Option<String>,
}
json_struct!(FieldJson { name, ty => "type", source, description });

fn field_type(id: &str) -> Result<FieldType, String> {
    Ok(match id {
        "text" => FieldType::Text,
        "number" => FieldType::Number,
        "bool" => FieldType::Bool,
        "date" => FieldType::Date,
        _ => {
            return Err(format!(
                "Bilinmeyen alan türü “{id}”: text, number, bool ya da date olmalı."
            ));
        }
    })
}

pub fn type_id(ty: FieldType) -> &'static str {
    match ty {
        FieldType::Text => "text",
        FieldType::Number => "number",
        FieldType::Bool => "bool",
        FieldType::Date => "date",
    }
}

pub fn source_id(source: FieldSource) -> &'static str {
    match source {
        FieldSource::User => "user",
        FieldSource::Attribute => "attribute",
        FieldSource::Builtin => "builtin",
    }
}

fn schema(fields: Vec<FieldJson>) -> Result<Schema, String> {
    let fields = fields
        .into_iter()
        .map(|f| {
            Ok(FieldDef {
                ty: field_type(&f.ty)?,
                source: match f.source.as_deref() {
                    Some("user") => FieldSource::User,
                    Some("builtin") => FieldSource::Builtin,
                    None | Some("attribute") => FieldSource::Attribute,
                    Some(other) => {
                        return Err(format!(
                            "Bilinmeyen alan kaynağı “{other}”: user, attribute ya da builtin olmalı."
                        ));
                    }
                },
                name: f.name,
                description: f.description.unwrap_or_default(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Schema { fields })
}

struct SpanJson {
    class: &'static str,
    start: usize,
    end: usize,
}
json_struct!(out SpanJson { class, start, end });

struct DiagnosticJson {
    start: usize,
    end: usize,
    message: String,
    text: String,
}
json_struct!(out DiagnosticJson { start, end, message, text });

impl From<Diagnostic> for DiagnosticJson {
    fn from(d: Diagnostic) -> DiagnosticJson {
        DiagnosticJson {
            text: d.text(),
            start: d.start,
            end: d.end,
            message: d.message,
        }
    }
}

struct CheckJson {
    error: Option<DiagnosticJson>,
    warnings: Vec<DiagnosticJson>,
}
json_struct!(out CheckJson { error, warnings });

struct ItemJson {
    kind: &'static str,
    label: String,
    detail: String,
    insert: String,
    caret: usize,
    key: String,
    alias: Option<String>,
}
json_struct!(out ItemJson { kind, label, detail, insert, caret, key, alias });

impl From<Item> for ItemJson {
    fn from(i: Item) -> ItemJson {
        ItemJson {
            kind: i.kind.id(),
            label: i.label,
            detail: i.detail,
            insert: i.insert,
            caret: i.caret,
            key: i.key,
            alias: i.alias,
        }
    }
}

struct CompletionJson {
    start: usize,
    end: usize,
    items: Vec<ItemJson>,
}
json_struct!(out CompletionJson { start, end, items });

struct SignatureArgJson {
    name: &'static str,
    description: &'static str,
    optional: bool,
    start: usize,
    end: usize,
}
json_struct!(out SignatureArgJson { name, description, optional, start, end });

struct SignatureJson {
    key: String,
    name: &'static str,
    signature: &'static str,
    description: &'static str,
    args: Vec<SignatureArgJson>,
    active: Option<usize>,
    open: usize,
}
json_struct!(out SignatureJson { key, name, signature, description, args, active, open });

struct BracketJson {
    at: usize,
    partner: Option<usize>,
}
json_struct!(out BracketJson { at, partner });

struct ArgJson {
    name: String,
    description: String,
    optional: bool,
}
json_struct!(out ArgJson { name, description, optional });

struct ExampleJson {
    expression: String,
    result: String,
}
json_struct!(out ExampleJson { expression, result });

struct HelpJson {
    key: String,
    kind: &'static str,
    title: String,
    group: &'static str,
    signature: String,
    description: String,
    args: Vec<ArgJson>,
    examples: Vec<ExampleJson>,
    aliases: Vec<String>,
    ty: Option<&'static str>,
    source: Option<&'static str>,
}
json_struct!(out HelpJson { key, kind, title, group, signature, description, args, examples, aliases, ty => "type", source });

impl From<Help> for HelpJson {
    fn from(h: Help) -> HelpJson {
        HelpJson {
            key: h.key,
            kind: h.kind.id(),
            title: h.title,
            group: h.group,
            signature: h.signature,
            description: h.description,
            args: h
                .args
                .into_iter()
                .map(
                    |Arg {
                         name,
                         description,
                         optional,
                     }| ArgJson {
                        name,
                        description,
                        optional,
                    },
                )
                .collect(),
            examples: h
                .examples
                .into_iter()
                .map(|(expression, result)| ExampleJson { expression, result })
                .collect(),
            aliases: h.aliases,
            ty: h.field.map(|(ty, _)| type_id(ty)),
            source: h.field.map(|(_, s)| source_id(s)),
        }
    }
}

struct SectionJson {
    group: &'static str,
    title: &'static str,
    items: Vec<ItemJson>,
}
json_struct!(out SectionJson { group, title, items });

impl From<Section> for SectionJson {
    fn from(s: Section) -> SectionJson {
        SectionJson {
            group: s.group.id(),
            title: s.title,
            items: s.items.into_iter().map(ItemJson::from).collect(),
        }
    }
}

struct ValueJson {
    text: String,
    insert: String,
}
json_struct!(out ValueJson { text, insert });

impl From<ValueItem> for ValueJson {
    fn from(v: ValueItem) -> ValueJson {
        ValueJson {
            text: v.text,
            insert: v.insert,
        }
    }
}

/// The text after placing an entry, and where the cursor goes.
struct Placed {
    text: String,
    caret: usize,
}
json_struct!(out Placed { text, caret });

fn tokens(source: &str) -> Vec<SpanJson> {
    editor::tokens(source)
        .into_iter()
        .map(|s| SpanJson {
            class: s.class.id(),
            start: s.start,
            end: s.end,
        })
        .collect()
}

fn check(source: &str, fields: Vec<FieldJson>) -> Result<CheckJson, String> {
    let c = editor::check(source, &schema(fields)?);
    Ok(CheckJson {
        error: c.error.map(DiagnosticJson::from),
        warnings: c.warnings.into_iter().map(DiagnosticJson::from).collect(),
    })
}

fn complete(
    source: &str,
    cursor: usize,
    fields: Vec<FieldJson>,
    explicit: bool,
) -> Result<Option<CompletionJson>, String> {
    Ok(
        editor::complete(source, cursor, &schema(fields)?, explicit).map(|c| CompletionJson {
            start: c.start,
            end: c.end,
            items: c.items.into_iter().map(ItemJson::from).collect(),
        }),
    )
}

fn signature(source: &str, cursor: usize) -> Option<SignatureJson> {
    editor::signature(source, cursor).map(|s| SignatureJson {
        key: s.key,
        name: s.name,
        signature: s.signature,
        description: s.description,
        args: s
            .args
            .into_iter()
            .map(|a| SignatureArgJson {
                name: a.name,
                description: a.description,
                optional: a.optional,
                start: a.start,
                end: a.end,
            })
            .collect(),
        active: s.active,
        open: s.open,
    })
}

/// A JSON value as the language's value (the preview's input).
fn value_of(v: &Json) -> Value<'_> {
    match v {
        Json::Bool(b) => Value::Bool(*b),
        Json::Num(x) => Value::Num(*x),
        Json::Str(s) => Value::Text(s.as_str().into()),
        _ => Value::Null,
    }
}

pub static OPS: &[Op] = &[
    op!("exprTokens", |source: String| tokens(&source)),
    op!("exprCheck", |source: String, fields: Vec<FieldJson>| check(
        &source, fields
    )),
    op!("exprComplete", |source: String,
                         cursor: usize,
                         fields: Vec<FieldJson>,
                         explicit: bool| {
        complete(&source, cursor, fields, explicit)
    }),
    op!("exprSignature", |source: String, cursor: usize| signature(
        &source, cursor
    )),
    op!("exprBracket", |source: String, cursor: usize| {
        editor::bracket(&source, cursor).map(|b| BracketJson {
            at: b.at,
            partner: b.partner,
        })
    }),
    op!("exprHelp", |key: String, fields: Vec<FieldJson>| {
        schema(fields).map(|s| editor::help(&key, &s).map(HelpJson::from))
    }),
    op!(
        "exprHelpAt",
        |source: String, cursor: usize, fields: Vec<FieldJson>| {
            schema(fields).map(|s| editor::help_at(&source, cursor, &s).map(HelpJson::from))
        }
    ),
    op!(
        "exprBuilderCatalog",
        |fields: Vec<FieldJson>, query: String| {
            schema(fields).map(|s| {
                editor::catalog(&s, &query)
                    .into_iter()
                    .map(SectionJson::from)
                    .collect::<Vec<_>>()
            })
        }
    ),
    op!("exprPlace", |source: String,
                      start: usize,
                      end: usize,
                      kind: String,
                      insert: String,
                      caret: usize| {
        editor::Kind::from_id(&kind)
            .map(|kind| {
                let (text, caret) = editor::place(&source, start, end, kind, &insert, caret);
                Placed { text, caret }
            })
            .ok_or_else(|| format!("Bilinmeyen öğe türü “{kind}”."))
    }),
    op!("exprValues", |values: Vec<String>, ty: String| {
        field_type(&ty).map(|ty| {
            editor::values(&values, ty)
                .into_iter()
                .map(ValueJson::from)
                .collect::<Vec<_>>()
        })
    }),
    // Any JSON value: written out, as op! reads typed arguments.
    Op {
        name: "exprPreview",
        run: |args| match Json::parse(args)? {
            Json::Arr(list) => Ok(kentos_geometry_core::api::json::to_string(
                &editor::preview(&value_of(list.first().unwrap_or(&Json::Null))),
            )),
            _ => Err("exprPreview: argümanlar bir dizi olmalı.".into()),
        },
    },
];

/// The id of an operation in this table, or None.
pub fn find(name: &str) -> Option<usize> {
    OPS.iter().position(|o| o.name == name)
}

/// Runs operation `id` of this table on JSON arguments.
pub fn run(id: usize, args: &str) -> Result<String, String> {
    match OPS.get(id) {
        Some(op) => (op.run)(args),
        None => Err(format!("İfade çekirdeğinde {id} numaralı işlem yok.")),
    }
}

/// Runs an operation by name (the fixture).
pub fn run_named(name: &str, args: &str) -> Result<String, String> {
    match find(name) {
        Some(id) => run(id, args),
        None => Err(format!("İfade çekirdeğinde “{name}” işlemi yok.")),
    }
}
