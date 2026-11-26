//! A form field's list source in the designer (the CBS review's 3rd item, docs/adr/9322): the chosen item's layer field,
//! linked to a project list through `cad.domains.edit`'s `link` (the command Alanlar's Projenin listesi runs), and the
//! offer of every source by name: the layer field's own list, a project list, another layer.

use kentos_commands::{CommandResult, DomainsEdit, DomainsOp, FieldDomain, FormItem, LayerField, LayerFieldKind};
use kentos_native_application::{ExecutionContext, domains_edit};

use super::{Window, find};
use crate::app::App;

/// What the field's list is now: the project list it is linked to, its own codes, or none.
pub(crate) enum FieldList<'a> {
    Linked(&'a FieldDomain),
    Own(usize),
    None,
}

impl App {
    /// The chosen item's layer field when it is a field: its layer, key, label and kind.
    pub(crate) fn chosen_list_field(
        &self,
        w: &Window,
        fields: &[LayerField],
    ) -> Option<(String, String, String, LayerFieldKind)> {
        let form = self.project_forms().iter().find(|f| w.chosen.as_deref() == Some(f.id.as_str()))?;
        let item = find(&w.body.as_ref()?.items, w.item.as_deref()?)?;
        let FormItem::Field(f) = item else {
            return None;
        };
        let field = fields.iter().find(|x| x.key() == f.field)?;
        Some((form.layer.clone(), field.key().to_owned(), field.label().to_owned(), field.kind))
    }

    /// The layer field `field` of `layer` linked to the project list `domain`: one undo step, the field's codes the
    /// list's from then on; why not, in the command's words.
    pub(crate) fn link_field_domain(&mut self, layer: &str, field: &str, domain: &str) -> Result<(), String> {
        let Some(doc) = self.document.as_mut() else {
            return Err("Açık çizim yok.".into());
        };
        let input = DomainsEdit {
            edit: DomainsOp::Link { layer: layer.to_owned(), field: field.to_owned(), domain: Some(domain.to_owned()) },
            expected_revision: None,
        };
        match domains_edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { .. } => {
                let name = doc
                    .model
                    .settings()
                    .domains
                    .iter()
                    .find(|d| d.id == domain)
                    .map_or_else(|| domain.to_owned(), |d| d.name.clone());
                self.output(format!("Alan “{name}” listesine bağlandı; kodları bu listenin, Değer listeleri'nde düzenlenir."));
                Ok(())
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => Ok(()),
        }
    }

    /// The project lists a field of `kind` may link to, in the project's order.
    pub(crate) fn lists_for(&self, kind: LayerFieldKind) -> Vec<FieldDomain> {
        self.document
            .as_ref()
            .map(|d| d.model.settings().domains.iter().filter(|x| x.kind == kind).cloned().collect())
            .unwrap_or_default()
    }

    /// What `field`'s list is now.
    pub(crate) fn field_list<'a>(&self, field: &LayerField, lists: &'a [FieldDomain]) -> FieldList<'a> {
        if let Some(d) = field.domain.as_deref().and_then(|id| lists.iter().find(|x| x.id == id)) {
            return FieldList::Linked(d);
        }
        match field.values.as_ref().map_or(0, Vec::len) {
            0 => FieldList::None,
            n => FieldList::Own(n),
        }
    }
}

/// A project list's name in a choice: its codes' count or its range, so that lists are told apart.
pub(crate) fn list_words(d: &FieldDomain) -> String {
    if d.coded() {
        format!("{} ({} kod)", d.name, d.values.as_ref().map_or(0, Vec::len))
    } else {
        format!("{} ({} – {})", d.name, d.min.as_deref().unwrap_or("…"), d.max.as_deref().unwrap_or("…"))
    }
}

/// What a list holds in one line, shown under the source: the field's kind, the codes offered and those retired, and
/// whether the field may be left empty.
pub(crate) fn list_summary(field: &LayerField, list: Option<&FieldDomain>) -> String {
    let codes = list.and_then(|d| d.values.as_ref()).or(field.values.as_ref());
    let mut parts = vec![field.kind.label().to_owned()];
    match codes {
        Some(values) => {
            let shown: Vec<String> = values
                .iter()
                .filter(|c| !c.retired)
                .take(4)
                .map(|c| if c.label.is_empty() { c.code.clone() } else { format!("{} {}", c.code, c.label) })
                .collect();
            let active = values.iter().filter(|c| !c.retired).count();
            let retired = values.len() - active;
            let more = if active > shown.len() { ", …" } else { "" };
            parts.push(format!("{active} etkin kod ({}{more})", shown.join(", ")));
            if retired > 0 {
                parts.push(format!("{retired} kullanımdan kalkmış (eski kayıtta okunur, seçilmez)"));
            }
        }
        None => match list {
            Some(d) => parts.push(format!("aralık {} – {}", d.min.as_deref().unwrap_or("…"), d.max.as_deref().unwrap_or("…"))),
            None => parts.push("listesi yok: değer serbest yazılır".to_owned()),
        },
    }
    parts.push(if field.required { "boş bırakılamaz".to_owned() } else { "boş bırakılabilir".to_owned() });
    parts.join(" · ")
}
