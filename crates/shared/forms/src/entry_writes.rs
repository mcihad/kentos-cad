//! What a Veri girişi may write of a record (docs/adr/0764 §1): the fields its layer's live forms show,
//! on a layer a live form writes. A field they do not show it never read: its absence keeps the stored
//! value (it writes what it reads, docs/adr/0411 §6); a value it gives that differs from the stored one
//! is refused, never kept silently. The server's guard and the desktop's document ask the same rule.

use crate::reach::Reach;
use crate::record::Attrs;

/// Why a Veri girişi may not write a change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryRefusal {
    /// No live form writes the layer's records.
    NoForm,
    /// The first field (by name) the change writes that the layer's live forms do not show.
    Hidden(String),
}

impl EntryRefusal {
    /// The refusal in words, for the layer named `layer`.
    pub fn message(&self, layer: &str) -> String {
        match self {
            EntryRefusal::NoForm => format!(
                "“{layer}” katmanının yayımlanmış formu yok; Veri girişi bu katmanın kayıtlarını değiştiremez. Katmana bir form yayımlatın ya da proje yöneticisine başvurun."
            ),
            EntryRefusal::Hidden(field) => format!(
                "“{field}” alanı “{layer}” katmanının formlarında yok; Veri girişi onu yazamaz. Alanı formun yayınına ekletin ya da proje yöneticisine başvurun."
            ),
        }
    }
}

/// Why `reach`'s Veri girişi may not change a `layer` record's attributes from `before` to `after`;
/// none when it may (or nothing changes).
pub fn entry_write_refusal(
    reach: &Reach,
    layer: &str,
    before: &Attrs,
    after: &Attrs,
) -> Option<EntryRefusal> {
    let hidden = |k: &str| !reach.level(layer, k).readable();
    let changed: Vec<&String> = before
        .keys()
        .chain(after.keys())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .filter(|k| {
            if hidden(k) {
                after.get(*k).is_some_and(|v| before.get(*k) != Some(v))
            } else {
                before.get(*k) != after.get(*k)
            }
        })
        .collect();
    if changed.is_empty() {
        return None;
    }
    if !reach.through_live_form(layer) {
        return Some(EntryRefusal::NoForm);
    }
    changed
        .into_iter()
        .find(|k| hidden(k))
        .map(|k| EntryRefusal::Hidden(k.clone()))
}
