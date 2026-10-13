//! The four edge commands of docs/adr/0250 in every command's shape
//! (`validate`, `plan`, `execute` over their own input): Buda and Uzat are
//! [`crate::trim`] with their act, Köşe yuvarla and Pah [`crate::corner`]
//! with their size.

macro_rules! trim_like {
    ($name:ident, $act:expr, $id:path, $version:path, $doc:literal) => {
        #[doc = $doc]
        pub mod $name {
            use kentos_contracts::{
                CommandResult, EntitiesTrim, EntitiesTrimPlan, EntitiesTrimmed,
            };

            use crate::ExecutionContext;
            use crate::trim;

            pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesTrim) -> CommandResult<()> {
                trim::validate(cx, input, $act)
            }

            pub fn plan(
                cx: &ExecutionContext<'_>,
                input: &EntitiesTrim,
            ) -> CommandResult<EntitiesTrimPlan> {
                trim::plan(cx, input, $act)
            }

            pub fn execute(
                cx: &mut ExecutionContext<'_>,
                input: EntitiesTrim,
            ) -> CommandResult<EntitiesTrimmed> {
                crate::call::recorded(cx, $id, $version, input, |cx, input| {
                    trim::execute(cx, input, $act)
                })
            }
        }
    };
}

macro_rules! corner_like {
    ($name:ident, $input:ty, $id:path, $version:path, $doc:literal) => {
        #[doc = $doc]
        pub mod $name {
            use kentos_contracts::{CommandResult, EntitiesCornerPlan, EntitiesCornered};

            use crate::ExecutionContext;
            use crate::corner;

            pub fn validate(cx: &ExecutionContext<'_>, input: &$input) -> CommandResult<()> {
                corner::validate(cx, input.into())
            }

            pub fn plan(
                cx: &ExecutionContext<'_>,
                input: &$input,
            ) -> CommandResult<EntitiesCornerPlan> {
                corner::plan(cx, input.into())
            }

            pub fn execute(
                cx: &mut ExecutionContext<'_>,
                input: $input,
            ) -> CommandResult<EntitiesCornered> {
                crate::call::recorded(cx, $id, $version, input, |cx, input| {
                    corner::execute(cx, (&input).into())
                })
            }
        }
    };
}

trim_like!(
    entities_trim,
    crate::trim::Act::Trim,
    kentos_contracts::CAD_ENTITIES_TRIM,
    kentos_contracts::CAD_ENTITIES_TRIM_VERSION,
    "`cad.entities.trim` v1: Buda."
);
trim_like!(
    entities_extend,
    crate::trim::Act::Extend,
    kentos_contracts::CAD_ENTITIES_EXTEND,
    kentos_contracts::CAD_ENTITIES_EXTEND_VERSION,
    "`cad.entities.extend` v1: Uzat."
);
corner_like!(
    entities_fillet,
    kentos_contracts::EntitiesFillet,
    kentos_contracts::CAD_ENTITIES_FILLET,
    kentos_contracts::CAD_ENTITIES_FILLET_VERSION,
    "`cad.entities.fillet` v1: Köşe yuvarla."
);
corner_like!(
    entities_chamfer,
    kentos_contracts::EntitiesChamfer,
    kentos_contracts::CAD_ENTITIES_CHAMFER,
    kentos_contracts::CAD_ENTITIES_CHAMFER_VERSION,
    "`cad.entities.chamfer` v1: Pah."
);
