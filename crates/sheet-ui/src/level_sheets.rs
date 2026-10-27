//! Yeni ▾'s Kat planı paftaları (docs/adr/0718): a sheet per level of the project from the mode's default template,
//! its questions asked once for all of them, through the sheet core's `levels::level_sheets` (`cad.sheets.levels`
//! makes the same): every map on the levels' objects' box at the project's scale, its level the sheet's. One undo
//! step of the sheet mode.

use kentos_sheet::levels::{LevelSheetsOptions, level_sheets};
use kentos_sheet::template::{InstanceIds, InstanceOptions, Template, VariableValue};

use crate::designer::{Designer, Effect, Say};

/// The undo step's name.
pub const STEP: &str = "Kat planı paftaları";

impl Designer {
    /// Kat planı paftaları: the default template's questions first, when it has any.
    pub(crate) fn level_sheets(&mut self) -> Vec<Effect> {
        if self.ctx.levels.is_empty() {
            return vec![Effect::Say(
                Say::Warn,
                "Projede kat yok; önce Proje ayarları › Mimari'de kat ekleyin.".into(),
            )];
        }
        let id = self.profile.default_template.clone();
        let Some(t) = kentos_sheet::template::system_template(&id).cloned() else {
            return Vec::new();
        };
        if t.variables.is_empty() {
            return self.make_level_sheets(&t, Vec::new());
        }
        self.ask_then_levels(&t)
    }

    /// The sheets made with the template's answers `values`; the first one in front.
    pub(crate) fn make_level_sheets(&mut self, t: &Template, values: Vec<VariableValue>) -> Vec<Effect> {
        let options = InstanceOptions {
            paper: None,
            name: None,
            center: None,
            scale: self.ctx.capabilities.plot_scale,
            values,
        };
        let how = LevelSheetsOptions {
            bounds: self.ctx.level_bounds.or(self.ctx.drawing_bounds),
            ..Default::default()
        };
        // The sheet mode's own ids, made ahead: one set per level at most.
        let mut pool: Vec<InstanceIds> = (0..self.ctx.levels.len())
            .map(|_| InstanceIds {
                sheet: self.new_id("pafta"),
                master: t.master.as_ref().map(|_| self.new_id("ana")),
                items: t.sheet.items.iter().map(|_| self.new_id("o")).collect(),
                master_items: t
                    .master
                    .iter()
                    .flat_map(|m| m.items.iter())
                    .map(|_| self.new_id("ao"))
                    .collect(),
            })
            .collect();
        pool.reverse();
        let levels = self.ctx.levels.clone();
        let made = match level_sheets(&self.book, t, &options, &levels, &how, |sheet| {
            pool.pop().unwrap_or_else(|| kentos_sheet::create::ids_for(t, sheet))
        }) {
            Ok(made) => made,
            Err(e) => return vec![Effect::Say(Say::Warn, e.message)],
        };
        for p in &made.pictures {
            if let Some(bytes) = kentos_sheet::template::base64_decode(&p.data) {
                self.add_asset_bytes(&p.meta.sha256, bytes);
            }
        }
        let first = made.made.first().map(|m| m.sheet.clone());
        let numbers: Vec<&str> = made.made.iter().map(|m| m.number.as_str()).collect();
        let said = match (numbers.first(), numbers.last()) {
            (Some(a), Some(b)) if a != b => format!("{} kat planı paftası: {a} … {b}.", numbers.len()),
            (Some(a), _) => format!("Kat planı paftası: {a}."),
            _ => String::new(),
        };
        let kept = if made.kept.is_empty() {
            String::new()
        } else {
            format!(" Paftası olan {} kat atlandı.", made.kept.len())
        };
        if !self.commit_as(made.ops, Some(STEP)) {
            return Vec::new();
        }
        self.open = first;
        self.selection.clear();
        self.view = crate::view_math::View::default();
        self.refresh();
        vec![Effect::ShowSheet, Effect::Say(Say::Success, format!("{said}{kept}"))]
    }
}
