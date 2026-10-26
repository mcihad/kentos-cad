//! Only the settings an architectural package may change. Ordinary set_settings remains
//! outside model undo; explicit package installation records this narrow reversible part.
use crate::{Document, history::Op};
use kentos_contracts::{
    ArchLevel, DimensionStyleDef, DrawingUnit, SpaceMethod, TextStyleDef, WallStyleDef,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TemplateSettings {
    unit: Option<DrawingUnit>,
    scale: f64,
    text: Vec<TextStyleDef>,
    dimension: Vec<DimensionStyleDef>,
    walls: Vec<WallStyleDef>,
    levels: Vec<ArchLevel>,
    method: Option<SpaceMethod>,
}
impl TemplateSettings {
    fn of(doc: &Document) -> Self {
        let s = doc.settings();
        Self {
            unit: s.drawing_unit,
            scale: s.plot_scale,
            text: s.text_styles.clone(),
            dimension: s.dimension_styles.clone(),
            walls: s.wall_styles.clone(),
            levels: s.arch.as_ref().map_or_else(Vec::new, |a| a.levels.clone()),
            method: s.arch.as_ref().and_then(|a| a.space_method),
        }
    }
    /// Do not rewind settings that this package left untouched (or unrelated project preferences).
    pub(crate) fn apply_from(&self, before: &Self, doc: &mut Document) {
        let s = &mut doc.settings;
        if self.unit != before.unit {
            s.drawing_unit = self.unit;
        }
        if self.scale != before.scale {
            s.plot_scale = self.scale;
        }
        if self.text != before.text {
            s.text_styles.clone_from(&self.text);
        }
        if self.dimension != before.dimension {
            s.dimension_styles.clone_from(&self.dimension);
        }
        if self.walls != before.walls {
            s.wall_styles.clone_from(&self.walls);
        }
        if self.levels != before.levels || self.method != before.method {
            let mut arch = s.arch.clone().unwrap_or_default();
            if self.levels != before.levels {
                arch.levels.clone_from(&self.levels);
            }
            if self.method != before.method {
                arch.space_method = self.method;
            }
            s.arch = (!arch.is_empty()).then_some(arch);
        }
    }
}
impl Document {
    /// An explicit package's metadata belongs to its existing model group. On error the
    /// enclosing command/group reverts geometry and sheets; this helper restores its settings.
    pub fn with_template_settings<T, E>(
        &mut self,
        label: &str,
        body: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        let before = TemplateSettings::of(self);
        match body(self) {
            Ok(result) => {
                let after = TemplateSettings::of(self);
                if before != after {
                    self.record(
                        vec![Op::TemplateSettings {
                            before: Box::new(before),
                            after: Box::new(after),
                        }],
                        label,
                    );
                }
                Ok(result)
            }
            Err(error) => {
                let now = TemplateSettings::of(self);
                before.apply_from(&now, self);
                Err(error)
            }
        }
    }
}
