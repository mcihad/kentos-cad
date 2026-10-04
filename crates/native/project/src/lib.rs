//! KentOS's native project model (TODOS.md §3): the coordinate system
//! registry ([`crs`]), the project's systems as the transforms read them
//! ([`systems`], docs/adr/0168), its datum choices as Proje ayarları types
//! them ([`choice_form`]) and its own systems as Özel koordinat sistemi does
//! ([`definition_form`]), its survey settings as Proje ayarları › Ölçme
//! types them ([`survey_form`]), the drawing a new project starts as
//! ([`new_project`]) and what the Yeni proje wizard asks ([`wizard`]), the
//! web's `geo/crs.ts`, `model/newProject.ts` and `model/newProjectWizard.ts`.
//! The desktop and the headless command host (Python, AI) use the same.
#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod choice_form;
pub mod crs;
pub mod definition_form;
pub mod new_project;
pub mod provinces;
pub mod survey_form;
pub mod systems;
pub mod wizard;
