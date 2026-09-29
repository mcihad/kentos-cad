//! Kot ver (`tool.setElevation`, docs/adr/0142): gives the vertices of the
//! selected lines, polylines, areas (their holes too) and points their
//! elevations, in metres. The web's is `apps/web/src/tools/elevationTool.ts`;
//! the steps and the words are the same.
//!
//! - Objects are picked before or after ([`crate::modify`]): a selection made
//!   before the tool started skips the step. Enter or a right click ends the
//!   picking.
//! - Then a number typed writes it, in one of three ways:
//!   - Sabit: every vertex gets that value, and a point its `z`;
//!   - Artır (A), a toggle: every vertex that has an elevation gets it plus
//!     the difference typed; one without stays without, a point with a `z`
//!     is raised;
//!   - Sıfırla (S) writes at once: every vertex without an elevation (not 0),
//!     a point without `z`. With objects picked and not yet confirmed, they
//!     are the ones.
//! - After a write the tool is back at picking with nothing selected. Esc
//!   steps back one step: from the number to the picking (the selection
//!   stays), from picking out of the tool.
//!
//! What it writes goes through `cad.entities.edit` (operation `elevation`),
//! one undo step named “Kot ver”, one update for each object with its own
//! geometry and its elevations explicit. Objects on a locked layer are left
//! out and said; if only locked ones are chosen the command refuses them with
//! its own message. Objects that take no elevation are counted and said.
//!
//! Artır is kept for the run, not in [`crate::tool::Memory`]: the ribbon's
//! methods (Sabit, Artır, Sıfırla) each start the tool the way they say.

use kentos_contracts::{EditOperation, EntityEdit};
use kentos_domain::Slot;
use kentos_geometry_core::tools::point_text::js_trim;

use crate::Vec2;
use crate::edge;
use crate::elevation::{self, Change};
use crate::format::Format;
use crate::log::Level;
use crate::modify::{Modify, Stages};
use crate::points::plain_number;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow};

/// The tool's id: its command is `tool.setElevation`.
pub const ID: &str = "setElevation";
pub const LABEL: &str = "Kot ver";

/// What is said when nothing selected can hold an elevation.
pub const NONE_TAKES: &str =
    "Seçimde kot alan nesne yok; çizgi, çoklu çizgi, alan ya da nokta seçin.";
/// What is said when Artır has no elevation to raise.
pub const NOTHING_TO_RAISE: &str =
    "Seçili nesnelerin hiçbir köşesinde kot yok; fark eklenecek bir şey bulunamadı.";
/// What is said by Sıfırla before any object is picked.
pub const PICK_FIRST: &str = "Önce kotu silinecek nesneleri seçin.";

/// What a write does.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Give {
    /// Sabit: this elevation.
    Set(f64),
    /// Artır: this difference.
    Raise(f64),
    /// Sıfırla.
    Clear,
}

impl Give {
    fn change(self) -> Change {
        match self {
            Give::Set(z) => Change::Set(Some(z)),
            Give::Raise(by) => Change::Raise(by),
            Give::Clear => Change::Set(None),
        }
    }
}

/// A difference as the messages say it: with its sign, the minus a real one (U+2212).
fn signed(by: f64, f: &Format) -> String {
    if by < 0.0 {
        format!("−{}", f.length(-by))
    } else {
        format!("+{}", f.length(by))
    }
}

/// Kot ver's part after the selection.
#[derive(Clone, Debug, Default)]
pub struct SetElevation {
    /// Artır: the number typed is a difference to add, not the elevation.
    raise: bool,
    /// The selection holds nothing that takes an elevation: picking goes on.
    none_takes: bool,
    /// The tool goes back to picking, asked of the base after this call.
    back: bool,
}

impl SetElevation {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }

    /// The chips both steps offer: Artır, a toggle, and Sıfırla.
    fn chips(&self, prompt: Prompt) -> Prompt {
        prompt
            .toggle("Artır", "A", self.raise)
            .option("Sıfırla", "S")
    }

    /// The selected objects that still exist.
    fn selected(cx: &Context<'_>) -> Vec<Slot> {
        cx.selection
            .ids()
            .iter()
            .copied()
            .filter(|slot| cx.doc.get(*slot).is_some())
            .collect()
    }

    /// Writes `give` to the selection's objects that take an elevation and
    /// says what came of it; after a write the selection is cleared and the
    /// tool goes back to picking.
    fn write(&mut self, give: Give, cx: &mut Context<'_>) {
        let all = Self::selected(cx);
        let doc = &*cx.doc;
        let takers: Vec<Slot> = all
            .iter()
            .copied()
            .filter(|slot| doc.get(*slot).is_some_and(elevation::takes))
            .collect();
        if takers.is_empty() {
            cx.say(Level::Warn, NONE_TAKES);
            return;
        }
        let open: Vec<Slot> = takers
            .iter()
            .copied()
            .filter(|slot| doc.get(*slot).is_some_and(|e| edge::unlocked(e, doc)))
            .collect();
        // Only locked objects: the command refuses them with its own message.
        let chosen: Vec<Slot> = if open.is_empty() {
            takers.clone()
        } else if matches!(give, Give::Raise(_)) {
            open.iter()
                .copied()
                .filter(|slot| doc.get(*slot).is_some_and(elevation::has_any))
                .collect()
        } else {
            open.clone()
        };
        if chosen.is_empty() {
            cx.say(Level::Warn, NOTHING_TO_RAISE);
            return;
        }
        let changes: Vec<EntityEdit> = chosen
            .iter()
            .filter_map(|slot| {
                let geometry = elevation::geometry_with(doc.get(*slot)?, give.change())?;
                Some(EntityEdit::Update {
                    uid: edge::uid(doc, *slot),
                    geometry,
                })
            })
            .collect();
        if edge::write(EditOperation::Elevation, changes, cx).is_none() {
            return;
        }
        let n = chosen.len();
        let text = match give {
            Give::Raise(by) => format!("Kotlar {} değişti: {n} nesne.", signed(by, &cx.format())),
            Give::Clear => format!("Kot silindi: {n} nesne."),
            Give::Set(_) => format!("Kot verildi: {n} nesne."),
        };
        cx.say(Level::Info, text);
        if all.len() > takers.len() {
            cx.say(
                Level::Warn,
                format!(
                    "{} nesne kot almaz (yalnız çizgi, çoklu çizgi, alan ve nokta).",
                    all.len() - takers.len()
                ),
            );
        }
        if open.len() < takers.len() {
            cx.say(
                Level::Warn,
                format!(
                    "{} nesne kilitli katmanda olduğu için atlandı.",
                    takers.len() - open.len()
                ),
            );
        }
        // Done: back at the first step with nothing selected, for the next objects.
        cx.selection.clear();
        cx.selection.set_hover(None);
        self.back = true;
    }
}

impl Stages for SetElevation {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    /// The selection is confirmed: it must hold something that takes an
    /// elevation, or the tool says so and picking goes on.
    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        self.back = false;
        let doc = &*cx.doc;
        self.none_takes = !Self::selected(cx)
            .iter()
            .any(|slot| doc.get(*slot).is_some_and(elevation::takes));
        if self.none_takes {
            cx.say(Level::Warn, NONE_TAKES);
        }
        Flow::Stay
    }

    fn repick(&self) -> bool {
        self.none_takes
    }

    fn picking_prompt(&self, _n: usize) -> Option<Prompt> {
        Some(self.chips(Prompt::new(LABEL, "kot verilecek nesneleri seçin")))
    }

    /// While picking: A turns Artır over, S resets what is picked; a number
    /// means nothing before the objects are chosen.
    fn picking_input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        match upper_tr(js_trim(text)).as_str() {
            "A" => self.raise = !self.raise,
            "S" => {
                if cx.selection.is_empty() {
                    cx.say(Level::Warn, PICK_FIRST);
                } else {
                    cx.selection.set_hover(None);
                    self.write(Give::Clear, cx);
                }
            }
            _ => return false,
        }
        true
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn prompt(&self, _n: usize) -> Prompt {
        let step = if self.raise {
            "eklenecek farkı yazın (m)"
        } else {
            "kotu yazın (m)"
        };
        self.chips(Prompt::new(LABEL, step))
    }

    /// A click means nothing here: only a number does.
    fn point(&mut self, _p: Vec2, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    fn takes_points(&self) -> bool {
        false
    }

    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        match upper_tr(js_trim(text)).as_str() {
            "A" => self.raise = !self.raise,
            "S" => self.write(Give::Clear, cx),
            _ => {
                let n = plain_number(text)?;
                self.write(
                    if self.raise {
                        Give::Raise(n)
                    } else {
                        Give::Set(n)
                    },
                    cx,
                );
            }
        }
        Some(Flow::Stay)
    }

    /// What is not a number is not a point.
    fn typed_points(&self) -> bool {
        false
    }

    /// Esc: from the number back to picking, the selection kept.
    fn back(&mut self, _cx: &mut Context<'_>) -> bool {
        self.back = true;
        true
    }

    fn take_repick(&mut self) -> bool {
        std::mem::take(&mut self.back)
    }

    fn preview(&self, _hover: Vec2) -> Option<kentos_geometry_core::geom::affine::Affine> {
        None
    }
}
