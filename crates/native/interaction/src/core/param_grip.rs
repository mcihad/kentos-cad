//! A dynamic block's parameter grips under the select tool (docs/adr/0319 §1): a selected insert of a dynamic block
//! shows a grip for each parameter a hand sets (`Store::param_grips`). A linear's, a polar's, an angle's and a
//! point's grip is dragged, or clicked and placed with the next click; the value under the pointer keeps the
//! parameter's rule (`Store::param_value`), the insert's outline with it is the preview, and the placement writes
//! it through `cad.blocks.parameters` (`set`), one undo step. A flip's grip is clicked: the flip turns. A visibility
//! state and a lookup row are chosen in Öznitelikler (docs/adr/0319 §3).

use std::collections::BTreeMap;

use kentos_commands::{BlocksParameters, ParametersOperation};
use kentos_domain::{Slot, Uuid};
use kentos_geometry_core::block_dynamic::{GripKind, ParamGrip};
use kentos_geometry_core::entity::{ParamValue as CoreValue, Params, Shape};
use kentos_geometry_core::jsmath::js_hypot;
use kentos_native_application::{ExecutionContext, block_params};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::tool::{Context, Preview, Stroke, Tag};

/// How near a grip the pointer reaches it, pixels (the other grips').
const REACH: f64 = 6.0;

/// A parameter's grip being moved.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ParamDrag {
    pub slot: Slot,
    uid: Option<Uuid>,
    pub name: String,
    kind: GripKind,
    /// Where the grip was, and the point it is measured from.
    origin: Vec2,
    from: Option<Vec2>,
    /// A linear's direction on the drawing.
    along: Option<Vec2>,
    /// Where the button went down on it.
    pub down: [f64; 2],
    /// Clicked without dragging: the next click places it.
    pub hot: bool,
    /// Where the grip would go, and the value it would give there.
    at: Vec2,
    value: Option<CoreValue>,
    /// The insert with that value, dashed.
    ghosts: Vec<Stroke>,
}

/// The insert's kept values as the store holds them.
fn kept(slot: Slot, cx: &Context<'_>) -> Params {
    match cx
        .spatial
        .store()
        .get(f64::from(slot.0))
        .map(|it| &it.shape)
    {
        Some(Shape::Insert {
            params: Some(p), ..
        }) => p.clone(),
        _ => Params(Vec::new()),
    }
}

/// A core value as the command takes it.
fn value_of(v: &CoreValue) -> kentos_commands::ParamValue {
    use kentos_commands::ParamValue as V;
    match v {
        CoreValue::Flag(b) => V::Flag(*b),
        CoreValue::Number(x) => V::Number(*x),
        CoreValue::Text(t) => V::Text(t.clone()),
        CoreValue::Point(p) => V::Point(kentos_commands::Vec2 { x: p.x, y: p.y }),
    }
}

/// The parameter grip under the pointer of a selected insert not on a locked layer, the nearest within reach; none
/// past the grips' limit.
pub(crate) fn grip_at(screen: [f64; 2], cx: &Context<'_>) -> Option<(Slot, ParamGrip)> {
    let ids = cx.selection.ids();
    if ids.len() > crate::select::GRIP_LIMIT {
        return None;
    }
    let doc = &*cx.doc;
    let editable: Vec<Slot> = ids
        .iter()
        .copied()
        .filter(|&s| {
            doc.get(s).is_some_and(|e| {
                matches!(e, kentos_commands::Entity::Insert(_))
                    && !doc.layers().is_locked(&e.base().layer_id)
            })
        })
        .collect();
    let mut found = None;
    let mut best = REACH;
    for (slot, g) in cx.spatial.param_grips(&editable) {
        let s = cx.view.to_screen(g.at);
        let d = js_hypot(s[0] - screen[0], s[1] - screen[1]);
        if d <= best {
            best = d;
            found = Some((slot, g));
        }
    }
    found
}

/// Writes `values` on insert `slot` (`cad.blocks.parameters`, `set`); whether it was written.
fn write(
    slot: Slot,
    uid: Option<Uuid>,
    values: BTreeMap<String, kentos_commands::ParamValue>,
    cx: &mut Context<'_>,
) -> bool {
    let Some(uid) = uid.filter(|u| cx.doc.slot_of(*u) == Some(slot)) else {
        cx.say(
            Level::Warn,
            "Tutamacın bloğu artık çizimde yok (silinmiş ya da geri alınmış); tutamaç bırakıldı.",
        );
        return false;
    };
    let input = BlocksParameters {
        operation: ParametersOperation::Set,
        uids: vec![uid.to_string()],
        values: Some(values),
        names: None,
        expected_revision: None,
    };
    let result = block_params::execute(&mut ExecutionContext::new(cx.doc), input);
    points::written(result, cx).is_some()
}

/// A flip's grip clicked: its flip turns.
pub(crate) fn flip(slot: Slot, name: &str, cx: &mut Context<'_>) {
    let on = kept(slot, cx).of(name) == Some(&CoreValue::Flag(true));
    let uid = cx.doc.uid(slot);
    let values = BTreeMap::from([(name.to_owned(), kentos_commands::ParamValue::Flag(!on))]);
    if write(slot, uid, values, cx) {
        cx.say(
            Level::Success,
            format!("“{name}” {}.", if on { "kapatıldı" } else { "açıldı" }),
        );
    }
}

impl ParamDrag {
    pub fn start(slot: Slot, g: ParamGrip, down: [f64; 2], cx: &Context<'_>) -> Self {
        ParamDrag {
            slot,
            uid: cx.doc.uid(slot),
            name: g.name,
            kind: g.kind,
            origin: g.at,
            from: g.from,
            along: g.along,
            down,
            hot: false,
            at: g.at,
            value: None,
            ghosts: Vec::new(),
        }
    }

    /// The grip at `at`: the value it gives and the insert drawn with it.
    pub fn follow(&mut self, at: Vec2, cx: &Context<'_>) {
        self.at = at;
        self.value = cx.spatial.param_value(self.slot, &self.name, at);
        self.ghosts = match &self.value {
            Some(v) => {
                let values = kept(self.slot, cx).with(&self.name, v.clone());
                crate::modify::ghosts(&cx.spatial.varied_outlines(self.slot, &values)).0
            }
            None => Vec::new(),
        };
    }

    /// Places the grip where it follows: its value written; nothing where it gives none.
    pub fn commit(self, cx: &mut Context<'_>) {
        let Some(v) = &self.value else {
            cx.say(
                Level::Warn,
                format!(
                    "“{}” bu noktada bir değer almıyor; tutamaç yerinde bırakıldı.",
                    self.name
                ),
            );
            return;
        };
        let values = BTreeMap::from([(self.name.clone(), value_of(v))]);
        if write(self.slot, self.uid, values, cx) {
            cx.say(Level::Success, format!("“{}” değiştirildi.", self.name));
        }
    }

    /// The command line's words while the grip moves.
    pub fn prompt(&self) -> crate::prompt::Prompt {
        let what = match self.kind {
            GripKind::Linear => "uzunluğu ya da noktası",
            GripKind::Polar => "uzaklığı ya da noktası",
            GripKind::Angle => "açısının noktası",
            _ => "noktası",
        };
        crate::prompt::Prompt::new("Blok parametresi", format!("“{}” {what}", self.name))
    }

    /// A value typed for the grip: a linear's length, a polar's distance in the grip's direction, else a point; the
    /// rule keeps it as a dragged one. False when the text is neither.
    pub fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let from = self.from.unwrap_or(self.origin);
        // A linear's along its line, a polar's toward where the grip is.
        let unit = self.along.or_else(|| {
            let (dx, dy) = (self.at.x - from.x, self.at.y - from.y);
            let n = js_hypot(dx, dy);
            (n > 0.0).then(|| Vec2::new(dx / n, dy / n))
        });
        if matches!(self.kind, GripKind::Linear | GripKind::Polar)
            && let Some(d) = cx.typed_length(text)
        {
            // The length is the block's own: the insert's scale draws it larger.
            let scale = match cx
                .spatial
                .store()
                .get(f64::from(self.slot.0))
                .map(|it| &it.shape)
            {
                Some(Shape::Insert { scale, .. }) => scale.abs(),
                _ => 1.0,
            };
            if let Some(u) = unit {
                let p = Vec2::new(from.x + u.x * d * scale, from.y + u.y * d * scale);
                self.follow(p, cx);
                return true;
            }
        }
        match cx.typed_point(text, self.from, Some(self.at)) {
            Some(p) => {
                self.follow(p, cx);
                true
            }
            None => false,
        }
    }

    /// The insert with the value, a dashed line from the point the value is measured from, and its tag.
    pub fn preview(&self, format: &Format) -> Preview {
        let mut strokes = self.ghosts.clone();
        let from = self.from.unwrap_or(self.origin);
        strokes.push(Stroke::dashed(vec![from, self.at], false, [2.0, 3.0]));
        let said = match (&self.value, self.kind) {
            (Some(CoreValue::Number(x)), GripKind::Angle) => format.angle(*x),
            (Some(CoreValue::Number(x)), _) => format.length(*x),
            (Some(CoreValue::Point(p)), GripKind::Polar) => {
                format.length(js_hypot(p.x - from.x, p.y - from.y))
            }
            (Some(CoreValue::Point(_)), _) => format.length(js_hypot(
                self.at.x - self.origin.x,
                self.at.y - self.origin.y,
            )),
            _ => "değer yok".to_owned(),
        };
        Preview {
            strokes,
            tag: Some(Tag {
                at: self.at,
                lines: vec![format!("{}: {said}", self.name)],
            }),
            ..Preview::default()
        }
    }
}
