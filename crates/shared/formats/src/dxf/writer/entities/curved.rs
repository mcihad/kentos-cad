//! A text along a curve (docs/adr/0196 §5) in DXF: DXF has no such text
//! (AutoCAD's ARCTEXT is an Express Tools object other programs do not
//! read), so its letters are written as TEXTs in an anonymous block (*U),
//! each where it stands on the curve and turned as the curve there, in the
//! text's own frame; an INSERT places the block at the text's point, turned
//! as its frame. The INSERT carries the text's own fields in KentOS's data:
//! KentOS reads the text back (where the INSERT is now). In a block
//! definition the letters are its own TEXTs.

use kentos_contracts::{TextEntity, Vec2};
use kentos_geometry_core::entity::TextPlace;
use kentos_geometry_core::text::Font;

use super::super::super::xdata::{self, Meta};
use super::{Justified, Writer, app, text_value};
use crate::geom::v;

const WHAT: &str = "Eğri boyunca yazı";

/// A curved text's own fields as the contract's JSON, without the object's
/// (its layer, colour, label …, which the INSERT and KentOS's data say).
fn curve_json(t: &TextEntity) -> String {
    let mut value = serde_json::to_value(t).unwrap_or_default();
    if let Some(map) = value.as_object_mut() {
        for k in [
            "kind",
            "id",
            "layerId",
            "color",
            "attrs",
            "label",
            "symbol",
            "lineWeight",
        ] {
            map.remove(k);
        }
    }
    value.to_string()
}

impl Writer<'_> {
    /// A text along a curve as its letters' block and an INSERT (§5).
    pub(super) fn curved(&mut self, t: &TextEntity) -> bool {
        if t.text.trim().is_empty() {
            self.report.skip(WHAT, "boş yazı yazılmadı", 0);
            return false;
        }
        if !(t.height > 0.0) {
            self.report
                .skip(WHAT, "yazı yüksekliği sıfır ya da negatif; yazılmadı", 0);
            return false;
        }
        // In its own frame (its point the origin, unturned); measured as the file draws it (Arimo for
        // Arial) unless it has a typeface of its own.
        let local = TextEntity {
            p: v(0.0, 0.0),
            rotation: 0.0,
            ..t.clone()
        };
        let shape = crate::blocks::shape(&kentos_contracts::Entity::Text(local));
        let Some(along) = TextPlace::of(&shape).and_then(|p| p.along(Font::from_id("arimo")))
        else {
            return false;
        };
        let letters = along.letters();
        let chars: Vec<char> = t.text.chars().collect();
        let style = self.styles.text(&t.face).to_owned();
        // Where a letter of the frame is in the drawing.
        let (s, c) = crate::math::sin_cos_deg(t.rotation);
        let world = |q: Vec2| v(t.p.x + c * q.x - s * q.y, t.p.y + s * q.x + c * q.y);
        if self.defining {
            // A definition's letters are its own TEXTs: no block in a block, no KentOS data.
            let mut written = false;
            for (l, ch) in letters.iter().zip(&chars) {
                if ch.is_whitespace() {
                    continue;
                }
                let letter = TextEntity {
                    p: world(app(l.at)),
                    text: ch.to_string(),
                    rotation: t.rotation + l.turn,
                    align: None,
                    mask: false,
                    path: None,
                    ..t.clone()
                };
                written |= self.text(&letter, &t.base, Self::base_meta(&t.base));
            }
            self.report.note(
                WHAT,
                "blok tanımındaki eğri boyunca yazı harf harf yazı olarak yazıldı; KentOS'a harfleriyle geri okunur",
                0,
            );
            return written;
        }
        let record = self.handles.take();
        *self.anonymous += 1;
        let name = format!("*U{}", *self.anonymous);
        self.records.push((record, name.clone()));
        self.block_begin(record, &name);
        let mut changed = false;
        for (l, ch) in letters.iter().zip(&chars) {
            if ch.is_whitespace() {
                continue;
            }
            let (value, lost) = text_value(&ch.to_string());
            changed |= lost;
            self.block_text(
                record,
                &value,
                &Justified {
                    p: app(l.at),
                    height: t.height,
                    rotation: l.turn,
                    align: None,
                    width_factor: t.width_factor,
                },
                &style,
            );
        }
        self.block_end(record);
        if changed {
            self.report.note(
                WHAT,
                "harflerdeki denetim karakterleri boşluk oldu (DXF yazısı tek satırdır)",
                0,
            );
        }
        self.begin("INSERT", &t.base);
        self.out.str(100, "AcDbBlockReference");
        self.out.str(2, &name);
        self.out.xyz(10, t.p);
        if t.rotation != 0.0 {
            self.out.real(50, t.rotation);
        }
        for q in along.outline(&letters, 0.0) {
            self.grow(world(app(q)));
        }
        let mut meta = Self::base_meta(&t.base);
        meta.along = Some(curve_json(t));
        // What `end` would keep at most: the text and the data that is not the label, attributes or symbol.
        let bare = Meta {
            label: None,
            attrs: Default::default(),
            symbol: None,
            ..meta.clone()
        };
        if xdata::size(&xdata::groups(&bare)) > xdata::MAX_BYTES {
            meta.along = None;
            self.report.note(
                WHAT,
                "AutoCAD'in nesne başına genişletilmiş veri sınırını (16 KB) aştığı için yalnız harfleri olarak yazıldı; KentOS'a blok olarak geri okunur",
                0,
            );
        }
        self.report.note(
            WHAT,
            "DXF'te eğri boyunca yazı yok: harflerinden bir blok (*U) olarak yazıldı; başka programlar blok olarak gösterir, KentOS eğri boyunca yazı olarak geri okur",
            0,
        );
        if t.mask {
            self.report.note(
                WHAT,
                "zemini başka programlarda yoktur; KentOS'un verisinde kalır",
                0,
            );
        }
        self.end(meta);
        true
    }
}
