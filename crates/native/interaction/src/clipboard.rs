//! The clipboard (docs/adr/0056, 0309): what Kes, Panoya kopyala and Taban noktasıyla kopyala put aside for
//! Yapıştır, Özgün koordinatlara yapıştır and Blok olarak yapıştır.
//!
//! - It is the session's (CLAUDE.md §4.4), not the drawing's: it outlives the drawing on screen, so objects go from
//!   one drawing to another, and it stays in the app. The system clipboard is not used, as on the web.
//! - It holds a copy (`cad.entities.copy`'s [`Clip`]): the objects with the layers, blocks, styles and library items
//!   they use, the source's unit and a base point (the lower left corner of their box, or the one given).
//! - Kes deletes through `cad.entities.delete` (docs/adr/0029) inside one undo step “Kes”; objects on locked layers
//!   are not cut. The pastes write through `cad.entities.paste` (docs/adr/0309) in one undo step “Yapıştır”: each
//!   object to the layer at its layer's path (the missing made), the blocks and styles by name; a locked layer
//!   refuses all of it, a hidden one and another unit are said.

use std::collections::HashSet;
use std::convert::Infallible;

use kentos_contracts::{
    Clip, CommandResult, EntitiesCopy, EntitiesDelete, EntitiesPaste, Entity, LayerNode,
    LayerNodeType, PasteReport,
};
use kentos_domain::Slot;
use kentos_native_application::{ExecutionContext, delete, entities_copy, entities_paste};

use crate::Vec2;
use crate::log::Level;
use crate::points::wire;
use crate::tool::Context;

/// The undo steps' names (the web's).
pub const CUT_LABEL: &str = "Kes";
pub const PASTE_LABEL: &str = "Yapıştır";

/// The copy put aside.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Clipboard {
    clip: Option<Clip>,
}

impl Clipboard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Puts `clip` aside; what was there goes.
    pub fn set(&mut self, clip: Clip) {
        self.clip = Some(clip);
    }

    pub fn clip(&self) -> Option<&Clip> {
        self.clip.as_ref()
    }

    pub fn len(&self) -> usize {
        self.items().len()
    }

    pub fn is_empty(&self) -> bool {
        self.items().is_empty()
    }

    /// The objects, in the drawing's order.
    pub fn items(&self) -> &[Entity] {
        self.clip.as_ref().map_or(&[], |c| c.objects.as_slice())
    }

    /// Where the objects are placed from.
    pub fn base(&self) -> Vec2 {
        self.clip
            .as_ref()
            .map_or(Vec2::new(0.0, 0.0), |c| Vec2::new(c.base.x, c.base.y))
    }
}

/// The copy of `slots` from `cx`'s drawing, its base point `base` or their box's lower left corner.
pub fn copy_of(slots: &[Slot], base: Option<Vec2>, cx: &Context<'_>) -> Option<Clip> {
    let input = EntitiesCopy {
        uids: slots
            .iter()
            .filter_map(|s| cx.doc.uid(*s))
            .map(|u| u.to_string())
            .collect(),
        base: base.map(wire),
    };
    match entities_copy::answer(cx.doc, &input) {
        CommandResult::Completed { output, .. } => Some(output),
        _ => None,
    }
}

/// The selected objects that are in the drawing, in the order they were selected.
fn selected(cx: &Context<'_>) -> Vec<(Slot, Entity)> {
    cx.selection
        .ids()
        .iter()
        .filter_map(|slot| cx.doc.get(*slot).map(|e| (*slot, e.clone())))
        .collect()
}

/// Panoya kopyala (`edit.copy`): the selected objects go to the clipboard; the drawing does not change. How many;
/// none, and nothing said, with no selection.
pub fn copy(clipboard: &mut Clipboard, cx: &mut Context<'_>) -> usize {
    let slots: Vec<Slot> = selected(cx).into_iter().map(|(s, _)| s).collect();
    if slots.is_empty() {
        return 0;
    }
    let Some(clip) = copy_of(&slots, None, cx) else {
        return 0;
    };
    let n = clip.objects.len();
    clipboard.set(clip);
    cx.say(Level::Success, format!("{n} nesne panoya kopyalandı."));
    n
}

/// Kes (`edit.cut`): the selected objects on unlocked layers go to the clipboard and leave the drawing in one undo
/// step “Kes”. Those on locked layers stay, selected, and a warning counts them; when all are locked the clipboard
/// keeps what it had. How many were cut.
pub fn cut(clipboard: &mut Clipboard, cx: &mut Context<'_>) -> usize {
    let chosen = selected(cx);
    let all = chosen.len();
    let layers = cx.doc.layers();
    let editable: Vec<Slot> = chosen
        .into_iter()
        .filter(|(_, e)| !layers.is_locked(&e.base().layer_id))
        .map(|(s, _)| s)
        .collect();
    if editable.len() < all {
        cx.say(
            Level::Warn,
            format!(
                "{} nesne kilitli katmanda olduğu için kesilmedi.",
                all - editable.len()
            ),
        );
    }
    if editable.is_empty() {
        return 0;
    }
    let Some(clip) = copy_of(&editable, None, cx) else {
        return 0;
    };
    let n = editable.len();
    // Deleted through cad.entities.delete (docs/adr/0029), as Sil deletes; the step keeps the name “Kes”.
    let input = EntitiesDelete {
        uids: editable
            .iter()
            .filter_map(|slot| cx.doc.uid(*slot))
            .map(|uid| uid.to_string())
            .collect(),
        expected_revision: None,
    };
    let Ok(result) = cx.doc.transact(CUT_LABEL, |doc| {
        Ok::<_, Infallible>(delete::execute(&mut ExecutionContext::new(doc), input))
    });
    if crate::points::written(result, cx).is_none() {
        return 0;
    }
    clipboard.set(clip);
    let doc = &*cx.doc;
    cx.selection.retain(|slot| doc.get(slot).is_some());
    cx.say(Level::Success, format!("{n} nesne panoya kesildi."));
    n
}

/// Özgün koordinatlara yapıştır (`edit.pasteOriginal`): the clipboard's objects where they were copied, selected.
/// The new slots.
pub fn paste_in_place(clipboard: &Clipboard, cx: &mut Context<'_>) -> Vec<Slot> {
    let Some(clip) = clipboard.clip() else {
        return Vec::new();
    };
    let slots = paste(clip, None, None, cx);
    if !slots.is_empty() {
        cx.selection.set(slots.iter().copied());
    }
    slots
}

/// The paths of `nodes`' groups (“Kadastro”, “Kadastro / Yapı”).
fn group_paths(nodes: &[LayerNode], above: &str, out: &mut HashSet<String>) {
    for n in nodes {
        let here = if above.is_empty() {
            n.name.clone()
        } else {
            format!("{above} / {}", n.name)
        };
        if n.kind == LayerNodeType::Group {
            group_paths(&n.children, &here, out);
            out.insert(here);
        }
    }
}

/// What a paste brought besides its objects, said after them: “Kadastro grubu ve 2 katman açıldı (…)” …; `groups`
/// the drawing's group paths.
fn brought(r: &PasteReport, groups: &HashSet<String>) -> Vec<String> {
    let list = |v: &[String]| v.join(", ");
    let mut out = Vec::new();
    if let Some(b) = &r.block {
        out.push(format!("“{b}” bloğu olarak"));
    }
    let (made_groups, layers): (Vec<String>, Vec<String>) =
        r.layers.iter().cloned().partition(|p| groups.contains(p));
    let made = match (made_groups.len(), layers.len()) {
        (0, 0) => None,
        (0, n) => Some(format!("{n} katman")),
        (1, 0) => Some(format!("“{}” grubu", made_groups[0])),
        (g, 0) => Some(format!("{g} grup")),
        (1, n) => Some(format!("“{}” grubu ve {n} katman", made_groups[0])),
        (g, n) => Some(format!("{g} grup ve {n} katman")),
    };
    if let Some(made) = made {
        if layers.is_empty() {
            out.push(format!("{made} açıldı"));
        } else {
            out.push(format!("{made} açıldı ({})", list(&layers)));
        }
    }
    if !r.blocks.is_empty() {
        out.push(format!(
            "{} blok eklendi ({})",
            r.blocks.len(),
            list(&r.blocks)
        ));
    }
    if !r.kept.is_empty() {
        out.push(format!(
            "{} blok bu çizimin tanımıyla ({})",
            r.kept.len(),
            list(&r.kept)
        ));
    }
    if !r.styles.is_empty() {
        out.push(format!(
            "{} stil eklendi ({})",
            r.styles.len(),
            list(&r.styles)
        ));
    }
    out
}

/// Pastes `clip` through `cad.entities.paste` (docs/adr/0309): its base point at `at` (none: where it was copied),
/// as one block named `as_block` when given. The new slots; a refusal is said and nothing is written.
pub fn paste(
    clip: &Clip,
    at: Option<Vec2>,
    as_block: Option<String>,
    cx: &mut Context<'_>,
) -> Vec<Slot> {
    let input = EntitiesPaste {
        clip: clip.clone(),
        at: at.map(wire),
        as_block,
        expected_revision: None,
    };
    let result = entities_paste::execute(&mut ExecutionContext::new(cx.doc), input);
    let Some(out) = crate::points::written(result, cx) else {
        return Vec::new();
    };
    let slots: Vec<Slot> = out
        .created
        .iter()
        .filter_map(|u| kentos_domain::Uuid::parse_str(u).ok())
        .filter_map(|u| cx.doc.slot_of(u))
        .collect();
    let mut text = format!("{} nesne yapıştırıldı", clip.objects.len());
    let mut groups = HashSet::new();
    group_paths(cx.doc.layers().nodes(), "", &mut groups);
    let extra = brought(&out.report, &groups);
    if !extra.is_empty() {
        text.push_str(": ");
        text.push_str(&extra.join("; "));
    }
    text.push('.');
    cx.say(Level::Success, text);
    slots
}
