//! `cad.blocks.proxy` v1 (docs/adr/0319 §4): what an insert draws, as plain objects; the rules are
//! [`crate::block_variants`]'.

pub use crate::block_variants::{
    proxy_execute as execute, proxy_plan as plan, proxy_validate as validate,
};
