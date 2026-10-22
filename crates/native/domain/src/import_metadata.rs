//! An import's styles and paper book belong to its undo step. Ordinary sheet editing keeps its own history.
use crate::{Document, history::Op};
use kentos_contracts::{DimensionStyleDef, LineTypeDef, ProjectSheets, TextStyleDef};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ImportMetadata {
    text: Vec<TextStyleDef>,
    dimension: Vec<DimensionStyleDef>,
    line_types: Vec<LineTypeDef>,
    line_scale: Option<f64>,
    sheets: Option<Arc<ProjectSheets>>,
}
impl ImportMetadata {
    pub(crate) fn sheets(&self) -> Option<&ProjectSheets> {
        self.sheets.as_deref()
    }

    /// An import that changed only styles must leave an independently edited book alone.
    pub(crate) fn apply_from(&self, before: &Self, doc: &mut Document) {
        let sheets = doc.sheets.clone();
        self.install(doc);
        if self.sheets == before.sheets {
            doc.sheets = sheets;
        }
    }
    fn of(doc: &Document) -> Self {
        Self {
            text: doc.settings.text_styles.clone(),
            dimension: doc.settings.dimension_styles.clone(),
            line_types: doc.settings.line_types.clone(),
            line_scale: doc.settings.line_type_scale,
            sheets: doc.sheets.clone(),
        }
    }
    pub(crate) fn install(&self, doc: &mut Document) {
        doc.settings.text_styles.clone_from(&self.text);
        doc.settings.dimension_styles.clone_from(&self.dimension);
        doc.settings.line_types.clone_from(&self.line_types);
        doc.settings.line_type_scale = self.line_scale;
        doc.sheets.clone_from(&self.sheets);
    }
}
impl Document {
    /// Record only the imported tables/book, so undo does not revert unrelated project preferences.
    pub fn with_import_metadata<T, E>(
        &mut self,
        label: &str,
        body: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        let before = ImportMetadata::of(self);
        let book_in_group = self.group_open();
        match body(self) {
            Ok(result) => {
                let mut after = ImportMetadata::of(self);
                if book_in_group {
                    // set_sheets already recorded every book transition. Do not
                    // record an overlapping snapshot in the import's style op.
                    after.sheets.clone_from(&before.sheets);
                }
                if before != after {
                    self.record(
                        vec![Op::ImportMetadata {
                            before: Box::new(before),
                            after: Box::new(after),
                        }],
                        label,
                    );
                }
                Ok(result)
            }
            Err(error) => {
                before.install(self);
                Err(error)
            }
        }
    }
}
