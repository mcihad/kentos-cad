//! Yeni sürüm oluştur (`tool.timeVersion`) and Sona erdir (`tool.timeEnd`),
//! docs/adr/0210 §7; the web's is `apps/web/src/tools/timeVersionTool.ts`,
//! the steps and the words the same.
//!
//! - Objects are picked before or after ([`crate::modify`]): a selection made
//!   before the tool started skips the step.
//! - Then the date: the time slider's moment (the store's window), or today's
//!   midnight with the slider closed; Enter writes with it, a date typed
//!   (05.03.2024, 2024-03-05) writes with that one.
//! - Every object must be on a ranged temporal layer (a start and an end
//!   field), on a layer that is not locked, with dates that read, its start
//!   before the date and its end after it; otherwise nothing is written and
//!   the reason is said.
//! - Sona erdir: each object's end becomes the date. Yeni sürüm oluştur: so
//!   too, and beside each a copy with every property, starting at the date
//!   and ending where the object ended; the copies are selected. One undo step
//!   named after the tool, through `cad.entities.set` and `cad.entities.create`.
//! - A layer whose start or end field is of the date kind (docs/adr/0199)
//!   takes the day alone: the date is that day's midnight (§3).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI32, Ordering};

use kentos_contracts::{
    EntitiesCreate, EntitiesSetProperties, LayerFieldKind, LayerTime, PropertiesOperation,
};
use kentos_domain::Slot;
use kentos_geometry_core::time::{self, Read, Unit, Window};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::{ExecutionContext, create, set};

use crate::Vec2;
use crate::layer_move::new_object;
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

    /// Writes the date to the selection's objects (and their new versions); says what came of it.
    fn write(&mut self, cx: &mut Context<'_>) {
        let list = Self::selected(cx);
        if list.is_empty() {
            self.back = true;
            return;
        }
        let d = self.date;
        let when = time::show(d, Unit::Day);
        let doc = &*cx.doc;
        let layers = doc.layers();
        // The objects by layer, in the order their layers are first met.
        let mut groups: Vec<(String, LayerTime, f64, Vec<Slot>)> = Vec::new();
        let mut outside = 0;
        for slot in &list {
            let Some(e) = doc.get(*slot) else {
                continue;
            };
            let layer = e.base().layer_id.as_str();
            let node = layers.get(layer);
            let name = node.map_or(layer, |n| n.name.as_str());
            let Some(rule) = node
                .and_then(|n| n.time.clone())
                .filter(|t| t.end.is_some())
            else {
                cx.say(
                    Level::Warn,
                    format!(
                        "“{name}” katmanı başlangıç ve bitiş alanı olan bir zamansal katman değil; sürüm yalnız aralıklı katmanda yapılır. Katmanın zaman ayarlarına bakın. Hiçbir nesne yazılmadı."
                    ),
                );
                return;
            };
            if layers.is_locked(layer) {
                cx.say(
                    Level::Warn,
                    format!("“{name}” katmanı kilitli; hiçbir nesne yazılmadı. Kilidi Katmanlar panelinden açın."),
                );
                return;
            }
            let attrs = &e.base().attrs;
            let read = |k: &str| attrs.get(k).map_or(Read::Empty, |v| time::read(v));
            let (s, t) = (
                read(&rule.start),
                read(rule.end.as_deref().unwrap_or_default()),
            );
            if s == Read::Unreadable || t == Read::Unreadable {
                cx.say(
                    Level::Warn,
                    format!(
                        "“{name}” katmanındaki bir nesnenin başlangıcı ya da bitişi tarih olarak okunamıyor; önce Öznitelikler’den düzeltin. Hiçbir nesne yazılmadı."
                    ),
                );
                return;
            }
            let start = s.moment().unwrap_or(f64::NEG_INFINITY);
            let end = t.moment().unwrap_or(f64::INFINITY);
            // A date field of the layer's (docs/adr/0199) takes the day alone: the date is that day's midnight.
            let days = node.is_some_and(|n| {
                n.fields.iter().any(|f| {
                    f.kind == LayerFieldKind::Date
                        && (f.name == rule.start || Some(&f.name) == rule.end.as_ref())
                })
            });
            let at = if days { time::floor_to(d, Unit::Day) } else { d };
            if !(start < at && at < end) {
                outside += 1;
            }
            match groups.iter_mut().find(|(l, ..)| l == layer) {
                Some((.., slots)) => slots.push(*slot),
                None => groups.push((layer.to_owned(), rule, at, vec![*slot])),
            }
        }
        if outside > 0 {
            cx.say(
                Level::Warn,
                format!(
                    "{outside} nesnenin zamanı {when} anını içermiyor: başlangıcı bu tarihten önce, bitişi sonra olmalı. Hiçbir nesne yazılmadı."
                ),
            );
            return;
        }
        let label = self.label();
        let group = cx.doc.begin_group(label);
        let mut copies: Vec<Slot> = Vec::new();
        for (layer, rule, at, slots) in &groups {
            let text = time::write(*at, false);
            let end = rule.end.clone().unwrap_or_default();
            // The copies as the objects are before their end changes: each starts at the date and ends where the
            // object ended (an object without an end makes one without).
            let objects: Vec<_> = slots
                .iter()
                .filter_map(|s| cx.doc.get(*s))
                .filter_map(|e| {
                    let mut o = new_object(e)?;
                    let mut attrs = e.base().attrs.clone();
                    attrs.insert(rule.start.clone(), text.clone());
                    if !e.base().attrs.contains_key(&end) {
                        attrs.remove(&end);
                    }
                    o.attrs = Some(attrs);
                    Some(o)
                })
                .collect();
            let input = EntitiesSetProperties {
                uids: slots
                    .iter()
                    .filter_map(|s| cx.doc.uid(*s))
                    .map(|u| u.to_string())
                    .collect(),
                layer_id: None,
                color: None,
                line_weight: None,
                symbol: None,
                attrs: Some(BTreeMap::from([(end.clone(), Some(text.clone()))])),
                label: None,
                operation: PropertiesOperation::Attributes,
                expected_revision: None,
                unlink: false,
            };
            let result = set::execute(&mut ExecutionContext::new(cx.doc), input);
            if points::written(result, cx).is_none() {
                cx.doc.cancel_group(group);
                return;
            }
            if !self.version {
                continue;
            }
            let input = EntitiesCreate {
                layer_id: layer.clone(),
                objects,
                operation: None,
                expected_revision: None,
            };
            let result = create::execute(&mut ExecutionContext::new(cx.doc), input);
            let Some(output) = points::written(result, cx) else {
                cx.doc.cancel_group(group);
                return;
            };
            copies.extend(output.ids.into_iter().map(Slot));
        }
        cx.doc.end_group(group);
        let n = list.len();
        if self.version {
            let made = copies.len();
            cx.selection.set(copies);
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
