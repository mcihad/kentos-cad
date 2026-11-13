//! Nesnenin üstüne and Nesnenin altına (docs/adr/0310 §3), on the
//! selection-first base ([`crate::modify`]): the selected objects (picked first when nothing is selected), then the
//! object they go right above or under, clicked. They move through `cad.entities.order` with that reference, one undo
//! step named after the tool; the command's refusals (another layer, a locked layer) are said and the tool waits for
//! another reference.

use kentos_commands::{CommandResult, EntitiesOrder, OrderTo};
use kentos_geometry_core::geom::affine::Affine;
use kentos_native_application::{ExecutionContext, entities_order};

use crate::Vec2;
use crate::format::Format;
use crate::spatial::Spatial;
use crate::log::Level;
use crate::modify::{Modify, Stages};
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Pointer};

/// Nesnenin üstüne's id: its command is `tool.drawOrderAbove`.
pub const ABOVE_ID: &str = "drawOrderAbove";
/// Nesnenin altına's id: its command is `tool.drawOrderUnder`.
pub const UNDER_ID: &str = "drawOrderUnder";

/// Nesnenin üstüne or altına, after the selection.
#[derive(Clone, Debug)]
pub struct DrawOrder {
    to: OrderTo,
    /// The hovered object's kind, for the tag.
    over: Option<&'static str>,
}

impl DrawOrder {
    pub fn above() -> Modify<Self> {
        Modify::with(Self {
            to: OrderTo::Above,
            over: None,
        })
    }

    pub fn under() -> Modify<Self> {
        Modify::with(Self {
            to: OrderTo::Under,
            over: None,
        })
    }

    /// The selection right above or under the object at `p`; the tool leaves when the command wrote (or found
    /// nothing to change), and waits when it refused.
    fn to_object_at(&self, p: Vec2, cx: &mut Context<'_>) -> Flow {
        let Some(reference) = cx.spatial.pick(p, cx.pick_tolerance()) else {
            cx.say(Level::Warn, "Üstüne ya da altına konacak nesneye tıklayın.");
            return Flow::Stay;
        };
        let doc = &*cx.doc;
        let uids: Vec<String> = cx
            .selection
            .ids()
            .iter()
            .filter(|s| **s != reference)
            .filter_map(|s| doc.uid(*s))
            .map(|u| u.to_string())
            .collect();
        let Some(r) = doc.uid(reference) else {
            return Flow::Stay;
        };
        if uids.is_empty() {
            cx.say(
                Level::Warn,
                "Başvuru seçili nesnelerden biri olamaz; başka bir nesneye tıklayın.",
            );
            return Flow::Stay;
        }
        let input = EntitiesOrder {
            uids,
            to: self.to,
            reference: Some(r.to_string()),
            expected_revision: None,
        };
        let what = entities_order::label(self.to);
        match entities_order::execute(&mut ExecutionContext::new(cx.doc), input) {
            CommandResult::Completed { output, warnings } => {
                for w in warnings {
                    cx.say(Level::Warn, w.message);
                }
                let n = output.changed.len();
                if n == 0 {
                    cx.say(Level::Info, format!("{what}: nesneler zaten yerinde."));
                } else {
                    cx.say(
                        Level::Success,
                        format!("{what}: {n} nesnenin çizim sırası değişti."),
                    );
                }
                Flow::Exit
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                cx.say(Level::Warn, error.message);
                Flow::Stay
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => Flow::Exit,
        }
    }
}

impl Stages for DrawOrder {
    fn id(&self) -> &'static str {
        match self.to {
            OrderTo::Under => UNDER_ID,
            _ => ABOVE_ID,
        }
    }

    fn label(&self) -> &'static str {
        entities_order::label(self.to)
    }

    fn begin(&mut self, _cx: &mut Context<'_>) -> Flow {
        self.over = None;
        Flow::Stay
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn snaps(&self) -> bool {
        false
    }

    fn prompt(&self, _n: usize) -> Prompt {
        let way = match self.to {
            OrderTo::Under => "altına",
            _ => "üstüne",
        };
        Prompt::new(
            self.label(),
            format!("seçili nesnelerin {way} konacağı nesneye tıklayın"),
        )
    }

    /// A typed point: the object there is the reference.
    fn point(&mut self, p: Vec2, cx: &mut Context<'_>) -> Flow {
        self.to_object_at(p, cx)
    }

    fn typed(&mut self, _text: &str, _cx: &mut Context<'_>) -> Option<Flow> {
        None
    }

    fn preview(&self, _hover: Vec2) -> Option<Affine> {
        None
    }

    fn tag(&self, _hover: Vec2, _format: &Format) -> Vec<String> {
        match self.over {
            Some(kind) => vec![format!("Başvuru: {kind}")],
            None => Vec::new(),
        }
    }

    /// A move lights the object under the cursor; a press makes it the reference.
    fn pointer(&mut self, p: &Pointer, down: bool, cx: &mut Context<'_>) -> Option<Flow> {
        if down {
            return Some(self.to_object_at(p.raw, cx));
        }
        let hit = cx.spatial.pick(p.raw, cx.pick_tolerance());
        cx.selection.set_hover(hit);
        self.over = hit
            .and_then(|slot| cx.doc.get(slot))
            .map(|e| crate::data_search::kind_name(e));
        None
    }
}

/// The kinds whose words are drawn over the scene (docs/adr/0310 §8): a mask under or over them never covers their
/// words, whatever their order.
const WORDS: [kentos_commands::SelectKind; 5] = [
    kentos_commands::SelectKind::Text,
    kentos_commands::SelectKind::Dimension,
    kentos_commands::SelectKind::Leader,
    kentos_commands::SelectKind::Table,
    kentos_commands::SelectKind::Tag,
];

/// The shown objects of `kinds` in the box `a`–`b` (touching it counts), as `cad.entities.select` finds them.
fn found_in(
    doc: &kentos_domain::Document,
    spatial: &Spatial,
    a: Vec2,
    b: Vec2,
    kinds: Vec<kentos_commands::SelectKind>,
) -> Vec<kentos_domain::Slot> {
    let query = kentos_commands::EntitiesSelect {
        model_scope: spatial.sheet_model_scope(),
        region: kentos_commands::SelectRegion::Crossing {
            a: kentos_commands::Vec2 { x: a.x.min(b.x), y: a.y.min(b.y) },
            b: kentos_commands::Vec2 { x: a.x.max(b.x), y: a.y.max(b.y) },
        },
        kinds: Some(kinds),
        layers: None,
    };
    kentos_native_application::entities_select::select(doc, spatial.store(), &query)
        .map(|f| f.slots)
        .unwrap_or_default()
}

/// What to add when a mask and words meet in the box `a`–`b`: the words stay readable over the mask, which neither
/// Arkaya gönder nor the layers' order changes; none when there are no words there, or (`need_mask`) no mask.
pub fn words_over_mask_note(
    doc: &kentos_domain::Document,
    spatial: &Spatial,
    a: Vec2,
    b: Vec2,
    need_mask: bool,
) -> Option<String> {
    let words = found_in(doc, spatial, a, b, WORDS.to_vec()).len();
    if words == 0 {
        return None;
    }
    if need_mask {
        let masked = found_in(doc, spatial, a, b, vec![kentos_commands::SelectKind::Polygon])
            .iter()
            .filter_map(|s| doc.get(*s))
            .any(|e| matches!(e, kentos_commands::Entity::Polygon(p) if p.base.look.wipeout.is_some()));
        if !masked {
            return None;
        }
    }
    Some(format!(
        "Maske yazıları örtmez: buradaki {words} yazı, ölçü, kılavuz, tablo ya da etiketin yazısı maskenin üstünde \
         okunmaya devam eder (çizim sırası bunu değiştirmez). Gizlemek için yazıyı taşıyın ya da katmanını kapatın."
    ))
}

/// Whether the object carries words drawn over the scene (a text, dimension, leader, table or tag).
pub fn carries_words(e: &kentos_commands::Entity) -> bool {
    use kentos_commands::Entity as E;
    matches!(e, E::Text(_) | E::Dimension(_) | E::Leader(_) | E::Table(_) | E::Tag(_))
}
