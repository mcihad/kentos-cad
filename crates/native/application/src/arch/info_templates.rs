//! A layer's templates (docs/adr/0225 §6): an action's target and the
//! information card's title and text, split by the expression core
//! (`kentos_expression::template`) and their expressions compiled once
//! (`$sıra` and `$ölçek` refused: a card does not change with a run or the
//! drawing's scale), then filled for objects with the language's values,
//! a batch at a time (the label texts' `values`). `cad.layers.info` checks
//! with them, the Bilgi tool, the actions and the dialog's preview fill with
//! them. The web's is `apps/web/src/model/infoTemplates.ts`.

use kentos_commands::Entity;
use kentos_expression::rows::As;
use kentos_expression::template::{self, Template};
use kentos_expression::{Expr, Schema, Unit, compile_with};
use kentos_geometry_core::entity::Shape;

use crate::label_texts::values;

/// A template split and its expressions compiled.
#[derive(Clone, Debug)]
pub struct CompiledTemplate {
    pub template: Template,
    pub exprs: Vec<Expr>,
}

/// `text` as a template, its expressions in `unit` (a card is a reading:
/// the project's drawing unit, `expression_unit`); why not as the dialogs
/// say it (“12. karakterde: …”, the position the template's own).
pub fn compile_template(text: &str, unit: Unit) -> Result<CompiledTemplate, String> {
    let t = template::parse(text).map_err(|e| e.text())?;
    let schema = Schema {
        unit,
        ..Schema::default()
    };
    let mut exprs = Vec::new();
    for (source, at) in t.expressions() {
        let e = compile_with(source, &schema)
            .map_err(|e| format!("{}. karakterde: {}", at + e.at - 1, e.message))?;
        if e.needs.index {
            return Err(format!("{at}. karakterde: Şablonda $sıra kullanılamaz."));
        }
        if e.needs.scale {
            return Err(format!("{at}. karakterde: Şablonda $ölçek kullanılamaz."));
        }
        exprs.push(e);
    }
    Ok(CompiledTemplate { template: t, exprs })
}

impl CompiledTemplate {
    /// Whether it reads the objects' geometry (their shapes are asked).
    pub fn reads_geometry(&self) -> bool {
        self.exprs.iter().any(|e| {
            let n = e.needs;
            n.measured || n.centroid || n.bounds || n.vertices
        })
    }

    /// Whether it asks for the objects' places in WGS 84.
    pub fn needs_place(&self) -> bool {
        self.template.needs_place()
    }

    /// Its text for each object of `list`: `layer_name` names a layer by its
    /// id (`$katman`), `shape(i)` is object `i`'s shape, `place(i)` its
    /// latitude and longitude (none: the places are empty).
    pub fn texts<'s>(
        &self,
        list: &[&Entity],
        layer_name: &dyn Fn(&str) -> String,
        shape: &dyn Fn(usize) -> Option<&'s Shape>,
        place: &dyn Fn(usize) -> Option<(f64, f64)>,
    ) -> Vec<String> {
        let columns: Vec<Vec<Option<String>>> = self
            .exprs
            .iter()
            .map(|e| values(e, list, layer_name, shape, As::Text))
            .collect();
        let mut row: Vec<Option<String>> = Vec::with_capacity(columns.len());
        (0..list.len())
            .map(|i| {
                row.clear();
                row.extend(columns.iter().map(|c| c.get(i).cloned().flatten()));
                self.template.fill(&row, place(i))
            })
            .collect()
    }
}
