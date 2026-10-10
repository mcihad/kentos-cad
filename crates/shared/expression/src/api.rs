//! The expression builder's services in the browser's call table
//! (docs/adr/0100 §5): `kentos-geometry-wasm` looks a name up here after the
//! geometry core's and the style core's tables. Arguments and results are
//! JSON; positions are UTF-16 units from 0. `fixtures/expression/v2/builder.json`
//! pins every operation's answers on both platforms, `flow.json` the flow's
//! (`exprFlow`, `exprFlowEdit`; docs/adr/0101).

use kentos_geometry_core::api::Op;
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::{json_struct, op};

use crate::editor::flow::{self, Edit, Flow, FlowNode, Port, Tree};
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

/// A `@` value as the page gives it: `{ name, value, description? }` (docs/adr/0214 §2.3).
struct VariableJson {
    name: String,
    value: Value<'static>,
    description: String,
}

impl FromJson for VariableJson {
    fn from_json(v: &Json) -> Result<Self, String> {
        let Json::Obj(entries) = v else {
            return Err("değişken bir nesne olmalı.".into());
        };
        let get = |k: &str| entries.iter().find(|(key, _)| key == k).map(|(_, v)| v);
        let name = match get("name") {
            Some(Json::Str(s)) => s.clone(),
            _ => return Err("değişkenin adı (name) metin olmalı.".into()),
        };
        Ok(VariableJson {
            name,
            value: get("value").map_or(Value::Null, |v| value_of(v).into_owned()),
            description: match get("description") {
                Some(Json::Str(s)) => s.clone(),
                _ => String::new(),
            },
        })
    }
}

/// What the page says of the objects: their fields as a list (the builder's
/// first form), or `{ fields, variables, world }` with the `@` values and
/// whether the functions that look at other layers are given (docs/adr/0214).
struct SchemaArg {
    fields: Vec<FieldJson>,
    variables: Vec<VariableJson>,
    world: bool,
}

impl FromJson for SchemaArg {
    fn from_json(v: &Json) -> Result<Self, String> {
        match v {
            Json::Arr(_) => Ok(SchemaArg {
                fields: Vec::<FieldJson>::from_json(v)?,
                variables: Vec::new(),
                world: false,
            }),
            Json::Obj(entries) => {
                let get = |k: &str| entries.iter().find(|(key, _)| key == k).map(|(_, v)| v);
                Ok(SchemaArg {
                    fields: match get("fields") {
                        Some(f) => Vec::<FieldJson>::from_json(f)?,
                        None => Vec::new(),
                    },
                    variables: match get("variables") {
                        Some(f) => Vec::<VariableJson>::from_json(f)?,
                        None => Vec::new(),
                    },
                    world: matches!(get("world"), Some(Json::Bool(true))),
                })
            }
            _ => Err("alanlar bir dizi ya da { fields, variables, world } olmalı.".into()),
        }
    }
}

fn schema(arg: SchemaArg) -> Result<Schema, String> {
    let SchemaArg {
        fields,
        variables,
        world,
    } = arg;
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
    Ok(Schema {
        fields,
        variables: variables
            .into_iter()
            .map(|v| crate::Variable {
                name: v.name,
                value: v.value,
                description: v.description,
            })
            .collect(),
        world,
    })
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

fn check(source: &str, fields: SchemaArg) -> Result<CheckJson, String> {
    let c = editor::check(source, &schema(fields)?);
    Ok(CheckJson {
        error: c.error.map(DiagnosticJson::from),
        warnings: c.warnings.into_iter().map(DiagnosticJson::from).collect(),
    })
}

fn complete(
    source: &str,
    cursor: usize,
    fields: SchemaArg,
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

/// A tree of the flow as the page keeps it: `{ text, at? }`.
struct TreeJson {
    text: String,
    at: Option<[f64; 2]>,
}
json_struct!(TreeJson { text, at });

impl From<TreeJson> for Tree {
    fn from(t: TreeJson) -> Tree {
        Tree::new(t.text, t.at.map(|[x, y]| (x, y)))
    }
}

impl From<Tree> for TreeJson {
    fn from(t: Tree) -> TreeJson {
        TreeJson {
            text: t.text,
            at: t.at.map(|(x, y)| [x, y]),
        }
    }
}

struct PortJson {
    name: String,
    ty: &'static str,
    optional: bool,
    from: Option<String>,
    note: Option<String>,
    removable: bool,
    y: f64,
}
json_struct!(out PortJson { name, ty => "type", optional, from, note, removable, y });

impl From<Port> for PortJson {
    fn from(p: Port) -> PortJson {
        PortJson {
            name: p.name,
            ty: p.ty.id(),
            optional: p.optional,
            from: p.from,
            note: p.note,
            removable: p.removable,
            y: p.y,
        }
    }
}

struct NodeJson {
    id: String,
    kind: &'static str,
    title: String,
    key: Option<String>,
    ty: &'static str,
    ports: Vec<PortJson>,
    grows: bool,
    negated: Option<bool>,
    text: String,
    whole: bool,
    error: Option<String>,
    warnings: Vec<String>,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}
json_struct!(out NodeJson { id, kind, title, key, ty => "type", ports, grows, negated, text, whole, error, warnings, x, y, w, h });

impl From<FlowNode> for NodeJson {
    fn from(n: FlowNode) -> NodeJson {
        NodeJson {
            id: n.id,
            kind: n.kind.id(),
            title: n.title,
            key: n.key,
            ty: n.ty.id(),
            ports: n.ports.into_iter().map(PortJson::from).collect(),
            grows: n.grows,
            negated: n.negated,
            text: n.text,
            whole: n.whole,
            error: n.error,
            warnings: n.warnings,
            x: n.x,
            y: n.y,
            w: n.w,
            h: n.h,
        }
    }
}

/// A flow as the page draws it; `head`, `row`, `value`: the rows of a node;
/// `column`: a node and the gap after it.
struct FlowJson {
    nodes: Vec<NodeJson>,
    texts: Vec<String>,
    error: Option<DiagnosticJson>,
    bounds: [f64; 4],
    head: f64,
    row: f64,
    value: f64,
    column: f64,
}
json_struct!(out FlowJson { nodes, texts, error, bounds, head, row, value, column });

impl From<Flow> for FlowJson {
    fn from(f: Flow) -> FlowJson {
        let (l, t, r, b) = f.bounds;
        FlowJson {
            nodes: f.nodes.into_iter().map(NodeJson::from).collect(),
            texts: f.texts,
            error: f.error.map(DiagnosticJson::from),
            bounds: [l, t, r, b],
            head: flow::HEAD,
            row: flow::ROW,
            value: flow::VALUE,
            column: flow::COLUMN,
        }
    }
}

/// A change as the page sends it, `{ op: "connect", from, to, port }` …
enum EditJson {
    Add {
        key: String,
        at: [f64; 2],
    },
    AddText {
        text: String,
        at: [f64; 2],
    },
    Connect {
        from: String,
        to: String,
        port: usize,
    },
    Disconnect {
        to: String,
        port: usize,
    },
    Remove {
        node: String,
    },
    SetNumber {
        node: String,
        value: f64,
    },
    SetText {
        node: String,
        value: String,
    },
    SetBool {
        node: String,
        value: bool,
    },
    SetNull {
        node: String,
    },
    SetField {
        node: String,
        name: String,
    },
    SetVariable {
        node: String,
        name: String,
    },
    SetOperator {
        node: String,
        symbol: String,
    },
    SetFunction {
        node: String,
        name: String,
    },
    SetNegated {
        node: String,
        value: bool,
    },
    SetFold {
        node: String,
        value: bool,
    },
    AddPort {
        node: String,
    },
    RemovePort {
        node: String,
        port: usize,
    },
    Move {
        tree: usize,
        at: [f64; 2],
    },
}
kentos_geometry_core::json_tagged!(EditJson, "op",
    Add => "add" { key, at },
    AddText => "addText" { text, at },
    Connect => "connect" { from, to, port },
    Disconnect => "disconnect" { to, port },
    Remove => "remove" { node },
    SetNumber => "setNumber" { node, value },
    SetText => "setText" { node, value },
    SetBool => "setBool" { node, value },
    SetNull => "setNull" { node },
    SetField => "setField" { node, name },
    SetVariable => "setVariable" { node, name },
    SetOperator => "setOperator" { node, symbol },
    SetFunction => "setFunction" { node, name },
    SetNegated => "setNegated" { node, value },
    SetFold => "setFold" { node, value },
    AddPort => "addPort" { node },
    RemovePort => "removePort" { node, port },
    Move => "move" { tree, at },
);

impl From<EditJson> for Edit {
    fn from(e: EditJson) -> Edit {
        match e {
            EditJson::Add { key, at: [x, y] } => Edit::Add { key, at: (x, y) },
            EditJson::AddText { text, at: [x, y] } => Edit::AddText { text, at: (x, y) },
            EditJson::Connect { from, to, port } => Edit::Connect { from, to, port },
            EditJson::Disconnect { to, port } => Edit::Disconnect { to, port },
            EditJson::Remove { node } => Edit::Remove { node },
            EditJson::SetNumber { node, value } => Edit::SetNumber { node, value },
            EditJson::SetText { node, value } => Edit::SetText { node, value },
            EditJson::SetBool { node, value } => Edit::SetBool { node, value },
            EditJson::SetNull { node } => Edit::SetNull { node },
            EditJson::SetField { node, name } => Edit::SetField { node, name },
            EditJson::SetVariable { node, name } => Edit::SetVariable { node, name },
            EditJson::SetOperator { node, symbol } => Edit::SetOperator { node, symbol },
            EditJson::SetFunction { node, name } => Edit::SetFunction { node, name },
            EditJson::SetNegated { node, value } => Edit::SetNegated { node, value },
            EditJson::SetFold { node, value } => Edit::SetFold { node, value },
            EditJson::AddPort { node } => Edit::AddPort { node },
            EditJson::RemovePort { node, port } => Edit::RemovePort { node, port },
            EditJson::Move { tree, at: [x, y] } => Edit::Move { tree, at: (x, y) },
        }
    }
}

/// The trees after a change, and the node to select.
struct EditedJson {
    trees: Vec<TreeJson>,
    focus: Option<String>,
}
json_struct!(out EditedJson { trees, focus });

/// A JSON value as the language's value (the preview's input).
fn value_of(v: &Json) -> Value<'_> {
    match v {
        Json::Bool(b) => Value::Bool(*b),
        Json::Num(x) => Value::Num(*x),
        Json::Str(s) => Value::Text(s.as_str().into()),
        _ => Value::Null,
    }
}

/// A schema as the page gives it (JSON: a field list, or `{ fields,
/// variables, world }`): the browser's evaluations take it with the source.
pub fn schema_of(json: &str) -> Result<Schema, String> {
    if json.is_empty() {
        return Ok(Schema::default());
    }
    schema(SchemaArg::from_json(&Json::parse(json)?)?)
}

struct NeedsJson {
    measured: bool,
    vertices: bool,
    kind: bool,
    layer: bool,
    label: bool,
    index: bool,
    id: bool,
    scale: bool,
    centroid: bool,
    bounds: bool,
}
json_struct!(out NeedsJson { measured, vertices, kind, layer, label, index, id, scale, centroid, bounds });

impl From<crate::Needs> for NeedsJson {
    fn from(n: crate::Needs) -> NeedsJson {
        NeedsJson {
            measured: n.measured,
            vertices: n.vertices,
            kind: n.kind,
            layer: n.layer,
            label: n.label,
            index: n.index,
            id: n.id,
            scale: n.scale,
            centroid: n.centroid,
            bounds: n.bounds,
        }
    }
}

/// What the layers an expression looks at must give (docs/adr/0214 §3).
struct WorldJson {
    own: bool,
    layers: Vec<String>,
    fields: Vec<String>,
    needs: NeedsJson,
}
json_struct!(out WorldJson { own, layers, fields, needs });

/// `exprCompileIn`: the fields and values an expression reads in a
/// context, what its calls to other layers read, the `@` names it did not
/// know; or why it does not compile.
struct CompiledJson {
    ok: bool,
    fields: Option<Vec<String>>,
    needs: Option<NeedsJson>,
    world: Option<WorldJson>,
    unknown: Option<Vec<String>>,
    error: Option<String>,
    at: Option<f64>,
}
json_struct!(out CompiledJson { ok, fields, needs, world, unknown, error, at });

fn compiled_in(source: &str, arg: SchemaArg) -> Result<CompiledJson, String> {
    let s = schema(arg)?;
    Ok(match crate::compile_with(source, &s) {
        Ok(e) => {
            let world = e.looks_around().then(|| {
                let w = e.world_needs();
                WorldJson {
                    own: w.own,
                    layers: w.layers,
                    fields: w.fields,
                    needs: w.needs.into(),
                }
            });
            CompiledJson {
                ok: true,
                fields: Some(e.fields.clone()),
                needs: Some(e.needs.into()),
                world,
                unknown: Some(e.unknown_variables.clone()),
                error: None,
                at: None,
            }
        }
        Err(e) => CompiledJson {
            ok: false,
            fields: None,
            needs: None,
            world: None,
            unknown: None,
            error: Some(e.message),
            at: Some(e.at as f64),
        },
    })
}

pub static OPS: &[Op] = &[
    op!("exprCompileIn", |source: String, context: SchemaArg| {
        compiled_in(&source, context)
    }),
    op!("exprTokens", |source: String| tokens(&source)),
    op!("exprCheck", |source: String, fields: SchemaArg| check(
        &source, fields
    )),
    op!("exprComplete", |source: String,
                         cursor: usize,
                         fields: SchemaArg,
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
    op!("exprHelp", |key: String, fields: SchemaArg| {
        schema(fields).map(|s| editor::help(&key, &s).map(HelpJson::from))
    }),
    op!("exprHelpAt", |source: String,
                       cursor: usize,
                       fields: SchemaArg| {
        schema(fields).map(|s| editor::help_at(&source, cursor, &s).map(HelpJson::from))
    }),
    op!("exprBuilderCatalog", |fields: SchemaArg, query: String| {
        schema(fields).map(|s| {
            editor::catalog(&s, &query)
                .into_iter()
                .map(SectionJson::from)
                .collect::<Vec<_>>()
        })
    }),
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
    op!("exprFlow", |trees: Vec<TreeJson>, fields: SchemaArg| {
        schema(fields).map(|s| {
            let trees: Vec<Tree> = trees.into_iter().map(Tree::from).collect();
            FlowJson::from(flow::flow(&trees, &s))
        })
    }),
    op!("exprFlowEdit", |trees: Vec<TreeJson>,
                         change: EditJson,
                         fields: SchemaArg| {
        schema(fields).and_then(|s| {
            let trees: Vec<Tree> = trees.into_iter().map(Tree::from).collect();
            flow::edit(&trees, &Edit::from(change), &s).map(|(trees, focus)| EditedJson {
                trees: trees.into_iter().map(TreeJson::from).collect(),
                focus,
            })
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
