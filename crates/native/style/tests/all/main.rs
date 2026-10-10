//! The style crate's integration tests as one binary: every module is a test
//! file of its own, the ones run alone (by `--test`) beside this folder.
//! One binary links once where each file was linked on its own.

mod classify;
mod designer;
mod kstil;
mod legend;
mod object_templates;
mod renderer_timing;
mod table_lines;
mod tally;
mod template_form;
