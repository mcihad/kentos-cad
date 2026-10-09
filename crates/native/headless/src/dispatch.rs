//! A catalog command by name: its JSON input read as the command's own
//! type, run by the desktop's handler, its `CommandResult` written back as
//! JSON (the catalog's wire form, `status`-tagged).

use kentos_contracts::{
    ArcCreate, BlocksDefine, BlocksEdit, CircleCreate, EntitiesArray, EntitiesCreate,
    EntitiesDelete, EntitiesEdit, EntitiesSetProperties, EntitiesTransform, LayersService,
    LineCreate, NetworkDefine, PointCreate, PolygonCreate, PolylineCreate,
};
use kentos_domain::Document;
use kentos_native_application::{
    DESKTOP_COMMANDS, ExecutionContext, arc, array, blocks_define, blocks_edit, circle, create,
    delete, edit, layers_service, line, network_define, point, polygon, polyline, set, transform,
};
use serde_json::Value;

use crate::HeadlessError;

/// The commands this host runs, by name and version: the desktop's
/// (`kentos_native_application::DESKTOP_COMMANDS`).
pub const DESKTOP: &[(&str, u32)] = DESKTOP_COMMANDS;

/// What a call does (TODOS.md CMD-04): `validate` and `plan` write nothing;
/// `execute` writes one undo step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Validate,
    Plan,
    Execute,
}

impl Op {
    pub fn parse(text: &str) -> Result<Self, HeadlessError> {
        match text {
            "validate" => Ok(Op::Validate),
            "plan" => Ok(Op::Plan),
            "execute" => Ok(Op::Execute),
            other => Err(HeadlessError::new(
                "invalid_input",
                format!("“{other}” bir çağrı biçimi değil: validate, plan ya da execute olmalı."),
            )),
        }
    }
}

/// Runs `op` of `command` (version `version`, or the host's when absent)
/// on `doc` with `input`; the command's whole answer.
pub fn run(
    doc: &mut Document,
    command: &str,
    version: Option<u32>,
    op: Op,
    input: Value,
) -> Result<Value, HeadlessError> {
    let Some(&(_, own)) = DESKTOP.iter().find(|(name, _)| *name == command) else {
        return Err(refusal(command));
    };
    if let Some(v) = version
        && v != own
    {
        return Err(HeadlessError::new(
            "unknown_version",
            format!("{command} için {v}. sürüm yok; bu sürüm {own}. sürümü çalıştırır."),
        ));
    }
    macro_rules! run {
        ($module:ident, $input:ty) => {{
            let input: $input = serde_json::from_value(input).map_err(|e| {
                HeadlessError::new(
                    "invalid_input",
                    format!("{command} girdisi okunamadı: {e}. Kataloğun girdi şemasına bakın."),
                )
            })?;
            match op {
                Op::Validate => {
                    serde_json::to_value($module::validate(&ExecutionContext::new(doc), &input))
                }
                Op::Plan => serde_json::to_value($module::plan(&ExecutionContext::new(doc), &input)),
                Op::Execute => {
                    serde_json::to_value($module::execute(&mut ExecutionContext::new(doc), input))
                }
            }
        }};
    }
    let answer = match command {
        kentos_contracts::CAD_POLYGON_CREATE => run!(polygon, PolygonCreate),
        kentos_contracts::CAD_LINE_CREATE => run!(line, LineCreate),
        kentos_contracts::CAD_POLYLINE_CREATE => run!(polyline, PolylineCreate),
        kentos_contracts::CAD_ENTITIES_DELETE => run!(delete, EntitiesDelete),
        kentos_contracts::CAD_POINT_CREATE => run!(point, PointCreate),
        kentos_contracts::CAD_CIRCLE_CREATE => run!(circle, CircleCreate),
        kentos_contracts::CAD_ARC_CREATE => run!(arc, ArcCreate),
        kentos_contracts::CAD_ENTITIES_TRANSFORM => run!(transform, EntitiesTransform),
        kentos_contracts::CAD_ENTITIES_EDIT => run!(edit, EntitiesEdit),
        kentos_contracts::CAD_ENTITIES_ARRAY => run!(array, EntitiesArray),
        kentos_contracts::CAD_ENTITIES_CREATE => run!(create, EntitiesCreate),
        kentos_contracts::CAD_ENTITIES_SET => run!(set, EntitiesSetProperties),
        kentos_contracts::CAD_BLOCKS_DEFINE => run!(blocks_define, BlocksDefine),
        kentos_contracts::CAD_BLOCKS_EDIT => run!(blocks_edit, BlocksEdit),
        kentos_contracts::CAD_LAYERS_SERVICE => run!(layers_service, LayersService),
        kentos_contracts::CAD_NETWORK_DEFINE => run!(network_define, NetworkDefine),
        other => return Err(refusal(other)),
    };
    answer.map_err(|e| {
        HeadlessError::new(
            "unwritable_result",
            format!("{command} sonucu JSON'a yazılamadı: {e}"),
        )
    })
}

/// Why a command is not run here: the server's, or unknown.
fn refusal(command: &str) -> HeadlessError {
    let listed = kentos_contracts::catalog::catalog()
        .commands
        .into_iter()
        .find(|c| c.id == command);
    match listed {
        Some(c) => HeadlessError::new(
            "server_command",
            format!(
                "{command} ({}) sunucuda çalışır, açık çizimde değil. Bir sunucu bağlantısıyla \
                 çağırın (Python'da kentos.cad.Connection).",
                c.title
            ),
        ),
        None => HeadlessError::new(
            "unknown_command",
            format!("“{command}” katalogda yok. Komutların listesi kataloğun kendisindedir."),
        ),
    }
}
