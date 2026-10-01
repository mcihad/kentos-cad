//! A leader as DXF (docs/adr/0146 §8): a LEADER through its vertices, with
//! a note the landing's end as its last vertex (a hookline, 75 = 1, on the
//! side 74 names along 211, the note's direction), its arrowhead drawn or
//! not (71) and its height (40); the note an MTEXT the LEADER names (340)
//! and that names it back (its reactor), where the leader's layout puts the
//! note, the middle of its left or right there (71 = 4 or 6), over the
//! drawing's background when the leader has a mask (90 = 3). Its open and
//! dot arrowheads are KentOS's data (another program draws a filled arrow);
//! so are its note and turn, exactly, when the MTEXT's notation and the
//! directions written cannot say them.

use kentos_contracts::{LeaderArrow, LeaderEntity, Vec2};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::geom::leader::layout;
use kentos_geometry_core::text::{Font, TextAlign, width_em};

use super::super::super::dimension::mtext_value;
use super::super::super::strings::mtext_lines;
use super::super::super::xdata::{Meta, caret_decode};
use super::{Writer, mtext_chunks};
use crate::geom::v;
use crate::math::{atan2, deg, hypot, sin_cos_deg};

const WHAT: &str = "Kılavuz";
const ARROW: &str = "açık ve nokta ok KentOS verisi olarak yazıldı; başka programlar dolu ok gösterir, KentOS geri okur";

/// The turn (degrees) a reader takes from the direction written for
/// `rotation` (an MTEXT's 11, a LEADER's 211): its angle, from 0 up to 360.
fn turn_read_back(rotation: f64) -> f64 {
    let (s, c) = sin_cos_deg(rotation);
    let l = hypot(c, s);
    let rot = deg(atan2(s / l, c / l));
    let rot = if (0.0..360.0).contains(&rot) {
        rot
    } else {
        rot.rem_euclid(360.0)
    };
    if rot >= 360.0 { 0.0 } else { rot }
}

/// What a reader takes as the note from `text` written in MTEXT's
/// notation: its first line that has something.
fn note_read_back(text: &str) -> Option<String> {
    mtext_lines(&caret_decode(&mtext_value(text)))
        .into_iter()
        .find(|line| !line.trim().is_empty())
}

impl Writer<'_> {
    pub(super) fn leader(&mut self, l: &LeaderEntity) -> bool {
        if l.pts.len() < 2 {
            self.report.skip(WHAT, "iki köşesi yok; yazılmadı", 0);
            return false;
        }
        if !(l.height > 0.0) {
            self.report
                .skip(WHAT, "yüksekliği sıfır ya da negatif; yazılmadı", 0);
            return false;
        }
        let note = l.text.as_deref().filter(|t| !t.trim().is_empty());
        let pts: Vec<CoreVec2> = l.pts.iter().map(|p| CoreVec2::new(p.x, p.y)).collect();
        let Some(laid) = layout(
            &pts,
            l.height,
            l.rotation,
            l.arrow.map(LeaderArrow::name),
            note.is_some(),
        ) else {
            return false;
        };
        let landing = laid.landing.map(|[_, end]| v(end.x, end.y));
        let me = self.begin("LEADER", &l.base);
        let note_handle = note.map(|_| self.handles.take());
        self.out.str(100, "AcDbLeader");
        self.out.str(3, "Standard");
        self.out.int(71, i64::from(l.arrow != Some(LeaderArrow::None)));
        self.out.int(72, 0);
        self.out.int(73, if note.is_some() { 0 } else { 3 });
        self.out.int(74, i64::from(laid.side > 0.0));
        self.out.int(75, i64::from(landing.is_some()));
        self.out.real(40, l.height);
        let width = note.map_or(0.0, |t| width_em(t, Font::from_id("arimo")) * l.height);
        self.out.real(41, width);
        self.out
            .int(76, (l.pts.len() + usize::from(landing.is_some())) as i64);
        for p in l.pts.iter().copied().chain(landing) {
            self.out.xyz(10, p);
            self.grow(p);
        }
        if let Some(h) = note_handle {
            self.out.handle(340, h);
        }
        let (s, c) = sin_cos_deg(l.rotation);
        self.out.xyz(211, v(c, s));
        let mut meta = Self::base_meta(&l.base);
        if matches!(l.arrow, Some(LeaderArrow::Open | LeaderArrow::Dot)) {
            meta.arrow = l.arrow.map(|a| a.name().to_owned());
            self.report.note("Kılavuz oku", ARROW, 0);
        }
        // Only what a reader would not get back as it is.
        meta.note = note
            .filter(|t| note_read_back(t).as_deref() != Some(*t))
            .map(str::to_owned);
        meta.note_turn = (turn_read_back(l.rotation) != l.rotation).then_some(l.rotation);
        self.end(meta);
        if let (Some(h), Some(text), Some(at), Some(align)) =
            (note_handle, note, laid.note_point, laid.note_align)
        {
            self.leader_note(h, me, l, text, v(at.x, at.y), align);
        }
        true
    }

    /// The note of the leader `leader` (its LEADER's handle) as an MTEXT.
    fn leader_note(
        &mut self,
        h: u64,
        leader: u64,
        l: &LeaderEntity,
        text: &str,
        at: Vec2,
        align: TextAlign,
    ) {
        self.head("MTEXT", &l.base, h, Some(leader));
        self.out.str(100, "AcDbMText");
        self.out.xyz(10, at);
        self.out.real(40, l.height);
        self.out.real(41, 0.0);
        // The middle of its left (4) or right (6), left to right.
        self.out
            .int(71, if align == TextAlign::MiddleRight { 6 } else { 4 });
        self.out.int(72, 1);
        mtext_chunks(self.out, &mtext_value(text));
        self.out.str(7, "Standard");
        let (s, c) = sin_cos_deg(l.rotation);
        self.out.xyz(11, v(c, s));
        if l.mask {
            // The drawing's background behind it (AutoCAD's “use the drawing background”).
            self.out.int(90, 3);
            self.out.int(63, 256);
            self.out.real(45, 1.5);
            self.out.int(441, 0);
        }
        self.grow(at);
        self.end(Meta::default());
    }
}
