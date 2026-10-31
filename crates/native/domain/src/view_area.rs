//! A named view's area as an undo step (docs/adr/0789 §1): the crop region a sheet's linked maps cut to. The step
//! keeps the view's key, its area before and after; undo and redo give the area back only to the view of that key,
//! while it is still there, and leave its name, recipe and the other views as they are now.

use kentos_contracts::{NamedView, Vec2, view_holds, view_key};

use crate::Refusal;
use crate::document::Document;
use crate::history::Op;

/// A view's area: its middle, width, height and twist.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewArea {
    pub center: Vec2,
    pub width: f64,
    pub height: f64,
    pub twist: f64,
}

impl ViewArea {
    /// The area of `v`.
    pub fn of(v: &NamedView) -> Self {
        Self {
            center: v.center,
            width: v.width,
            height: v.height,
            twist: v.twist,
        }
    }

    /// `v` with this area.
    pub(crate) fn put(self, v: &mut NamedView) {
        v.center = self.center;
        v.width = self.width;
        v.height = self.height;
        v.twist = self.twist;
    }
}

impl Document {
    /// Gives the named view of key `key` the area `area` as one change named `label` (into the open transaction, else
    /// an undo step of its own). Refused with nothing changed: no such view, or an area that does not hold
    /// (`view_holds`). Whether anything changed.
    pub fn set_view_area(&mut self, key: &str, area: ViewArea, label: &str) -> Result<bool, Refusal> {
        let Some(v) = self.settings.named_views.iter().find(|v| view_key(v) == key) else {
            return Err(Refusal(format!("“{key}” kimlikli bir adlı görünüm yok.")));
        };
        let mut next = v.clone();
        area.put(&mut next);
        if !view_holds(&next) {
            return Err(Refusal(format!("“{}” görünümünün alanı tutmuyor.", v.name)));
        }
        let before = ViewArea::of(v);
        if before == area {
            return Ok(false);
        }
        self.transact(label, |doc| {
            doc.record(
                vec![Op::ViewArea {
                    key: key.to_owned(),
                    before,
                    after: area,
                }],
                label,
            );
            Ok(true)
        })
    }

    /// The step's area on the view of `key`, when it is still there (history.rs).
    pub(crate) fn put_view_area(&mut self, key: &str, area: ViewArea) {
        if let Some(v) = self.settings.named_views.iter_mut().find(|v| view_key(v) == key) {
            area.put(v);
        }
    }
}
