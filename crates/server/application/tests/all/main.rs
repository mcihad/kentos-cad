//! The server's use-case tests as one program (docs/adr/9400 §5): each file of this folder is a module of it, so the
//! server's libraries are linked once and `common` is built once. A new test file is declared here. Every database
//! test has a database of its own (`kentos_postgres::testing::TestDb`), so the modules share no state.
//!
//! `disk_full.rs` stays a program of its own beside this folder: scripts/ops/disk_full_drill.sh builds and runs it alone,
//! on a small file system of its own.

mod common;

mod acceptance_projects;
mod access;
mod backup;
mod blocks;
mod carriage;
mod catalog;
mod changes;
mod identity;
mod job_cache_cases;
mod job_kinds_cases;
mod lifecycle;
mod people;
mod persistent_ids;
mod portal;
mod processing_job_cases;
mod processing_jobs;
mod project_arrays;
mod project_casework;
mod project_catalog;
mod project_checkpoints;
mod project_constraints;
mod project_convert;
mod project_dimension_sets;
mod project_duplicate;
mod project_files;
mod project_form_scope;
mod project_form_spatial;
mod project_forms;
mod project_import;
mod project_invitations;
mod project_lifecycle;
mod project_particulars;
mod project_reach;
mod project_roles;
mod project_sheets;
mod project_snapshot;
mod retention;
mod server_job_cache;
mod server_jobs;
mod sheet_templates;
