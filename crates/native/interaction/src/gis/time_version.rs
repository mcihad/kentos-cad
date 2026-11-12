//! Yeni sürüm oluştur (`tool.timeVersion`) and Sona erdir (`tool.timeEnd`),
//! docs/adr/0210 §7; the web's is `apps/web/src/tools/timeVersionTool.ts`,
//! the steps and the words the same.
//!
//! - Objects are picked before or after ([`crate::modify`]): a selection made
//!   before the tool started skips the step.
//! - Then the date: the time slider's moment (the store's window), or today's
//!   midnight with the slider closed; Enter writes with it, a date typed
//!   (05.03.2024, 2024-03-05) writes with that one.
//! - The write is `cad.time.version`'s (docs/adr/0622 §9): every object must
//!   be on a ranged temporal layer (a start and an end field), on a layer that
//!   is not locked, with dates that read, its start before the date and its
//!   end after it; otherwise nothing is written and the command's reason is
//!   said.
//! - Sona erdir: each object's end becomes the date. Yeni sürüm oluştur: so
//!   too, and beside each a copy with every property, starting at the date
//!   and ending where the object ended; the copies are selected. One undo step
//!   named after the tool.
//! - A layer whose start or end field is of the date kind (docs/adr/0199)
//!   takes the day alone: the date is that day's midnight (§3).

use std::sync::atomic::{AtomicI32, Ordering};

use kentos_commands::{TimeVersion as TimeVersionInput, TimeVersionKind};
use kentos_domain::Slot;
use kentos_geometry_core::time::{self, Read, Unit, Window};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::{ExecutionContext, time_version};

use crate::Vec2;
use crate::log::Level;
use crate::modify::{Modify, Stages};
use crate::points;
use crate::prompt::Prompt;
use crate::tool::{Context, Flow};

/// The tools' ids: their commands are `tool.timeVersion` and `tool.timeEnd`.
pub const VERSION_ID: &str = "timeVersion";
pub const END_ID: &str = "timeEnd";
pub const VERSION_LABEL: &str = "Yeni sürüm oluştur";
pub const END_LABEL: &str = "Sona erdir";

/// The device's offset from UTC, seconds east, for today's midnight (the host sets it from its time zone).
pub static LOCAL_OFFSET: AtomicI32 = AtomicI32::new(0);

/// The tools' part after the selection.
#[derive(Clone, Debug)]
pub struct TimeVersion {
    version: bool,
    /// The date the write uses: the slider's moment, today's midnight or one typed.
    date: f64,
    /// The tool goes back to picking, asked of the base after this call.
    back: bool,
}

impl TimeVersion {
    /// Yeni sürüm oluştur (`version`) or Sona erdir.
    pub fn tool(version: bool) -> Modify<Self> {
        Modify::with(Self {
            version,
            date: 0.0,
            back: false,
        })
    }

    fn label(&self) -> &'static str {
        if self.version {
            VERSION_LABEL
        } else {
            END_LABEL
        }
    }

    /// The slider's moment while it is open, else today's midnight as a date is written.
    fn default_date(cx: &Context<'_>) -> f64 {
        match cx.spatial.store().time_window() {
            Some(Window::Instant(a) | Window::Range(a, _)) => a,
            None => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_millis() as i64)
                    + i64::from(LOCAL_OFFSET.load(Ordering::Relaxed)) * 1000;
                time::floor_to(now as f64, Unit::Day)
            }
        }
    }

    /// The selected objects that still exist, in the selection's order.
    fn selected(cx: &Context<'_>) -> Vec<Slot> {
        cx.selection
            .ids()
            .iter()
            .copied()
            .filter(|slot| cx.doc.get(*slot).is_some())
            .collect()
    }

    /// Writes the date to the selection's objects (and their new versions) through `cad.time.version`; says what
    /// came of it.
    fn write(&mut self, cx: &mut Context<'_>) {
        let list = Self::selected(cx);
        if list.is_empty() {
            self.back = true;
            return;
        }
        let when = time::show(self.date, Unit::Day);
        let input = TimeVersionInput {
            uids: list
                .iter()
                .filter_map(|s| cx.doc.uid(*s))
                .map(|u| u.to_string())
                .collect(),
            at: time::write(self.date, false),
            kind: if self.version {
                TimeVersionKind::Version
            } else {
                TimeVersionKind::End
            },
            expected_revision: None,
        };
        let result = time_version::execute(&mut ExecutionContext::new(cx.doc), input);
        let Some(output) = points::written(result, cx) else {
            return;
        };
        let n = output.ended.len();
        if self.version {
            let made = output.ids.len();
            cx.selection.set(output.ids.into_iter().map(Slot));
            cx.say(
                Level::Success,
                format!("Yeni sürüm: {n} nesne {when} tarihinde sona erdi, {made} yeni sürümü yazıldı ve seçildi."),
            );
        } else {
            cx.selection.clear();
            cx.say(Level::Success, format!("Sona erdi: {n} nesne, {when}."));
        }
        self.back = true;
    }
}

impl Stages for TimeVersion {
    fn id(&self) -> &'static str {
        if self.version { VERSION_ID } else { END_ID }
    }

    fn label(&self) -> &'static str {
        TimeVersion::label(self)
    }

    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        self.back = false;
        self.date = Self::default_date(cx);
        Flow::Stay
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn prompt(&self, _n: usize) -> Prompt {
        Prompt::new(
            TimeVersion::label(self),
            format!(
                "tarih {}: Enter yazar, başka bir tarih yazılabilir",
                time::show(self.date, Unit::Day)
            ),
        )
    }

    /// A click means nothing here: only a date does.
    fn point(&mut self, _p: Vec2, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    fn takes_points(&self) -> bool {
        false
    }

    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        let text = js_trim(text);
        match time::read(text) {
            Read::Empty => return None,
            Read::Unreadable => cx.say(
                Level::Warn,
                format!(
                    "“{text}” bir tarih olarak okunamadı; 05.03.2024 ya da 2024-03-05 gibi yazın."
                ),
            ),
            Read::Moment(t) => {
                self.date = t;
                self.write(cx);
            }
        }
        Some(Flow::Stay)
    }

    /// What is not a date is not a point.
    fn typed_points(&self) -> bool {
        false
    }

    /// Esc: from the date back to picking, the selection kept.
    fn back(&mut self, _cx: &mut Context<'_>) -> bool {
        self.back = true;
        true
    }

    /// Enter (or a right click) writes with the date shown.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.write(cx);
        Flow::Stay
    }

    fn take_repick(&mut self) -> bool {
        std::mem::take(&mut self.back)
    }

    fn preview(&self, _hover: Vec2) -> Option<kentos_geometry_core::geom::affine::Affine> {
        None
    }
}
