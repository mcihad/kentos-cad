//! The style core's operations in the browser's call table (docs/adr/0008):
//! the WASM crate looks a name up here after the geometry core's table.
//! Arguments and results are JSON; the bulk evaluation of an expression has
//! a typed entry instead (`expr::rows`).

use kentos_geometry_core::api::Op;
use kentos_geometry_core::{json_struct, op};

use crate::expr::{self, Needs, library};

struct NeedsJson {
    measured: bool,
    vertices: bool,
    kind: bool,
    layer: bool,
    label: bool,
    index: bool,
    id: bool,
    scale: bool,
}
json_struct!(out NeedsJson { measured, vertices, kind, layer, label, index, id, scale });

impl From<Needs> for NeedsJson {
    fn from(n: Needs) -> NeedsJson {
        NeedsJson {
            measured: n.measured,
            vertices: n.vertices,
            kind: n.kind,
            layer: n.layer,
            label: n.label,
            index: n.index,
            id: n.id,
            scale: n.scale,
        }
    }
}

/// `exprCompile`: the fields and variables an expression reads, or why it does not compile.
struct Compiled {
    ok: bool,
    fields: Option<Vec<String>>,
    needs: Option<NeedsJson>,
    error: Option<String>,
    at: Option<f64>,
}
json_struct!(out Compiled { ok, fields, needs, error, at });

fn compiled(source: &str) -> Compiled {
    match expr::compile(source) {
        Ok(e) => Compiled {
            ok: true,
            fields: Some(e.fields),
            needs: Some(e.needs.into()),
            error: None,
            at: None,
        },
        Err(e) => Compiled {
            ok: false,
            fields: None,
            needs: None,
            error: Some(e.message),
            at: Some(e.at as f64),
        },
    }
}

struct FunctionEntry {
    name: &'static str,
    aliases: Vec<&'static str>,
    signature: &'static str,
    description: &'static str,
}
json_struct!(out FunctionEntry { name, aliases, signature, description });

struct VariableEntry {
    name: &'static str,
    aliases: Vec<&'static str>,
    description: &'static str,
}
json_struct!(out VariableEntry { name, aliases, description });

/// `exprCatalog`: the functions and variables, for the expression field's menus.
struct Catalog {
    functions: Vec<FunctionEntry>,
    variables: Vec<VariableEntry>,
}
json_struct!(out Catalog { functions, variables });

fn catalog() -> Catalog {
    Catalog {
        functions: library::FUNCTIONS
            .iter()
            .map(|f| FunctionEntry {
                name: f.name,
                aliases: f.aliases.to_vec(),
                signature: f.signature,
                description: f.description,
            })
            .collect(),
        variables: library::VARIABLES
            .iter()
            .map(|v| VariableEntry {
                name: v.name,
                aliases: v.aliases.to_vec(),
                description: v.description,
            })
            .collect(),
    }
}

pub static OPS: &[Op] = &[
    op!("exprCompile", |source: String| compiled(&source)),
    // No arguments: written out, as the op! macro takes at least one.
    Op {
        name: "exprCatalog",
        run: |_| kentos_geometry_core::api::result(&catalog()),
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
        None => Err(format!("Stil çekirdeğinde {id} numaralı işlem yok.")),
    }
}

/// Runs an operation by name (the fixtures).
pub fn run_named(name: &str, args: &str) -> Result<String, String> {
    match find(name) {
        Some(id) => run(id, args),
        None => Err(format!("Stil çekirdeğinde “{name}” işlemi yok.")),
    }
}
