//! KentOS's native project model (TODOS.md §3): the coordinate system
//! registry ([`crs`]), the drawing a new project starts as
//! ([`new_project`]) and what the Yeni proje wizard asks ([`wizard`]), the
//! web's `geo/crs.ts`, `model/newProject.ts` and `model/newProjectWizard.ts`.
//! The desktop and the headless command host (Python, AI) use the same.
#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod crs;
pub mod new_project;
pub mod provinces;
pub mod wizard;
