//! The chosen fixes previewed on the drawing (docs/adr/0732 §5): run on a reading copy of it, nothing written; the
//! objects they write outlined dashed as they would be, those they remove in the danger colour.

use std::collections::BTreeSet;

use kentos_commands::CallOp;
use kentos_domain::Slot;
use kentos_native_application::{ExecutionContext, registry};

use super::PREVIEW_LIMIT;
use crate::app::App;
use crate::bottom::BottomTab;

impl App {
    /// The chosen findings' fixes as they would draw (docs/adr/0732 §5), for the drawing's preview layer: none while
    /// the tab is not shown, the check is old or no chosen finding has a fix. Kept until the drawing or the choice
    /// changes.
    pub(crate) fn arch_audit_preview(&self) -> Option<kentos_interaction::Preview> {
        if !(self.command_expanded && self.bottom_tab == BottomTab::Audit) || self.arch_audit_stale() {
            return None;
        }
        let doc = self.document.as_ref()?;
        let keys = self
            .arch_audit_chosen()
            .iter()
            .filter(|f| f.fix.is_some())
            .map(|f| f.key.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        if keys.is_empty() {
            return None;
        }
        let (generation, revision) = (doc.model.generation(), doc.model.revision());
        if let Some((g, r, k, made)) = self.arch_audit.preview.borrow().as_ref()
            && *g == generation
            && *r == revision
            && *k == keys
        {
            return made.clone();
        }
        let made = self.arch_audit_preview_made();
        *self.arch_audit.preview.borrow_mut() = Some((generation, revision, keys, made.clone()));
        made
    }

    /// The fixes run on a reading copy of the drawing (nothing written): the objects they write outlined dashed in
    /// the accent as they would be, those they remove in the danger colour as they are, and a tag that counts them.
    fn arch_audit_preview_made(&self) -> Option<kentos_interaction::Preview> {
        use kentos_geometry_core::geom::affine::IDENTITY;
        use kentos_interaction::{Preview, Tag, Tone as Ink};
        use kentos_native_application::{blocks, geometry, geometry_store, walls};

        let doc = self.document.as_ref()?;
        let (command, input, _) = self.arch_audit_fix_call()?;
        let mut copy = doc.model.reading_copy();
        let mark = copy.change_mark();
        let answer = registry::run(&mut ExecutionContext::new(&mut copy), &command, CallOp::Execute, input).ok()?;
        if answer["status"] != "completed" {
            return None;
        }
        let touched: BTreeSet<Slot> = match copy.changes_since(mark) {
            kentos_domain::Changes::Slots(slots) => slots.iter().copied().collect(),
            kentos_domain::Changes::All => return None,
        };
        let (kept, gone): (Vec<Slot>, Vec<Slot>) = touched.into_iter().partition(|s| copy.get(*s).is_some());
        let gone: Vec<f64> = gone
            .into_iter()
            .filter(|s| doc.model.get(*s).is_some())
            .map(|s| f64::from(s.0))
            .collect();
        // The written objects in a store of their own, with the walls their openings are in.
        let written: Vec<&kentos_commands::Entity> = kept.iter().filter_map(|s| copy.get(*s)).collect();
        let hosts: Vec<&kentos_commands::Entity> = geometry_store::opening_hosts(&copy, written.iter().copied())
            .into_iter()
            .filter_map(|(_, host)| host)
            .map(|h| Slot(h as u32))
            .filter(|h| !kept.contains(h))
            .filter_map(|h| copy.get(h))
            .collect();
        let mut store = kentos_geometry_core::store::Store::new();
        store.set_font(geometry::drawing_font(copy.settings().drawing_font));
        store.set_blocks(blocks::core_blocks(copy.blocks()));
        store.set_wall_view(walls::view_of(copy.settings()));
        store.put_many(written.iter().chain(hosts.iter()).map(|e| geometry_store::record(e)));
        store.set_hosts(geometry_store::opening_hosts(&copy, written.iter().copied()));
        let ids: Vec<f64> = kept.iter().map(|s| f64::from(s.0)).collect();
        let (mut strokes, mut marks) =
            kentos_interaction::modify::ghosts(&store.transform_outlines(&ids, &[IDENTITY], PREVIEW_LIMIT));
        let (removed, removed_marks) = kentos_interaction::modify::ghosts(
            &self
                .spatial
                .store()
                .transform_outlines(&gone, &[IDENTITY], PREVIEW_LIMIT),
        );
        strokes.extend(removed.into_iter().map(|s| s.tone(Ink::Danger).width(2.0)));
        marks.extend(removed_marks);
        if strokes.is_empty() && marks.is_empty() {
            return None;
        }
        let at = strokes
            .iter()
            .flat_map(|s| s.pts.iter())
            .chain(marks.iter())
            .fold(None::<kentos_geometry_core::Vec2>, |far, p| match far {
                Some(f) if f.x >= p.x => Some(f),
                _ => Some(*p),
            })?;
        let mut lines = vec!["Düzeltme önizlemesi".to_owned()];
        if !ids.is_empty() {
            lines.push(format!("{} nesne değişir", ids.len()));
        }
        if !gone.is_empty() {
            lines.push(format!("{} nesne silinir", gone.len()));
        }
        Some(Preview {
            strokes,
            marks,
            tag: Some(Tag { at, lines }),
            tag_tone: if gone.is_empty() { Ink::Accent } else { Ink::Danger },
            ..Preview::default()
        })
    }
}
