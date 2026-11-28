//! The fields a caller who works through forms alone does not read (docs/adr/9150): a cloud project's Veri girişi
//! (docs/adr/0411 §6) reads what its layer's live forms show, and the server sends the other fields not at all. The
//! attribute table, Bilgi's card and what leaves the table leave them out and say so by name, so a field withheld is
//! never taken for an empty one (a NULL, an empty text). The rule is the shared reach resolver's (`kentos_forms::reach`,
//! docs/adr/0550), the one the server answers with.

use crate::app::App;

impl App {
    /// The fields of `layer`, by name, the caller does not read: none for anyone but one who works through forms.
    pub(crate) fn withheld_fields(&self, layer: &str) -> Vec<String> {
        let Some(doc) = self.document.as_ref() else {
            return Vec::new();
        };
        if doc.model.object_policy() != kentos_domain::ObjectPolicy::AttributesOnly {
            return Vec::new();
        }
        let fields_of = |l: &str| {
            doc.model
                .layers()
                .get(l)
                .map_or(&[][..], |n| n.fields.as_slice())
        };
        let reach = kentos_forms::reach::Reach::new(
            &doc.model.settings().forms,
            fields_of,
            &[],
            kentos_expression::Unit::METRE,
            kentos_forms::reach::Caller::through_forms(),
        );
        fields_of(layer)
            .iter()
            .filter(|f| !reach.level(layer, &f.name).readable())
            .map(|f| f.name.clone())
            .collect()
    }
}

/// The words for fields withheld from the caller.
pub(crate) fn withheld_words(fields: &[String]) -> String {
    format!(
        "Rolünüz bu alanları okumuyor, gösterilmiyor: {}",
        fields.join(", ")
    )
}

#[cfg(test)]
#[path = "role_fields_tests.rs"]
mod role_fields_tests;
