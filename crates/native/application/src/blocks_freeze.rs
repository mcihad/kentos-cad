//! `cad.blocks.freeze` v1 (docs/adr/0319 §4): a dynamic block's inserts made inserts of plain blocks drawing what they
//! drew, one undo step “Sabit bloğa çevir”; the rules are [`crate::block_variants`]'.

pub use crate::block_variants::{
    freeze_execute as execute, freeze_plan as plan, freeze_validate as validate,
};
