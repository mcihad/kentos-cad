//! KentOS's native project model (TODOS.md §3): the coordinate system
//! registry ([`crs`]) and the drawing a new project starts as
//! ([`new_project`]), the web's `geo/crs.ts` and `model/newProject.ts`.
//! The desktop and the headless command host (Python, AI) use the same.
#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod crs;
pub mod new_project;
pub mod provinces;
