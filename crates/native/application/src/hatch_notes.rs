//! What a step did to the texts the hatches leave open (docs/adr/0332 §2), said after any command: a text deleted
//! leaves its hatches' lists (`cutout_gone`), a hatch's own linked label is taken off its list (`hatch_text_cycle`).
//! The document's own rules did it (hatch_ties.rs); here they are only said.

use std::collections::HashSet;

use kentos_contracts::{CommandWarning, Entity, EntityId};
use kentos_domain::{Document, StepReport, Uuid};

use crate::codes;

/// The step's notes about the texts its hatches leave open, in the drawing's order of the hatches.
pub(crate) fn notes(doc: &Document, report: &StepReport) -> Vec<CommandWarning> {
    let removed: HashSet<Uuid> = report
        .objects
        .iter()
        .filter(|c| c.removed())
        .map(|c| c.uid)
        .collect();
    let (mut gone, mut cycles) = (Vec::new(), Vec::new());
    for c in &report.objects {
        let (Some(before), Some(after)) = (&c.before, &c.after) else {
            continue;
        };
        let (Entity::Hatch(b), Entity::Hatch(a)) = (&**before, &**after) else {
            continue;
        };
        let (Some(was), Some(now)) = (&b.assoc, &a.assoc) else {
            continue;
        };
        let kept: HashSet<&EntityId> = now.cutouts.iter().collect();
        for id in was.cutouts.iter().filter(|id| !kept.contains(id)) {
            let uid = Uuid::from_bytes(id.0);
            if removed.contains(&uid) {
                gone.push(c.uid.to_string());
            } else if doc
                .slot_of(uid)
                .and_then(|s| doc.get(s))
                .is_some_and(|e| matches!(e, Entity::Text(t) if t.label_of.is_some_and(|l| Uuid::from_bytes(l.0) == c.uid)))
            {
                cycles.push(c.uid.to_string());
            }
        }
    }
    let mut out = Vec::new();
    if !gone.is_empty() {
        out.push(CommandWarning {
            code: codes::CUTOUT_GONE.into(),
            message: format!(
                "{} taramanın boş bıraktığı yazı silindi; taramalar artık orayı da tarar.",
                gone.len()
            ),
            path: None,
        });
    }
    if !cycles.is_empty() {
        out.push(CommandWarning {
            code: codes::HATCH_TEXT_CYCLE.into(),
            message: "Taramanın kendi bağlı etiketi boş bırakılmaz: yeri taramadan okunur, boşluğu taramayı yeniden değiştirirdi (döngü). Etiket taramanın listesinden çıkarıldı.".into(),
            path: None,
        });
    }
    out
}
