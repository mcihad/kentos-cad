//! Dizi: the web's `ArrayTool` (`apps/web/src/tools/modifyTools.ts`) on its
//! `SelectionFirstTool` base ([`crate::modify`]), step for step
//! (docs/adr/0047):
//!
//! - the objects, then the rows and columns typed as `2,3` (a comma, a
//!   semicolon or a space between; Enter keeps the last ones);
//! - then the spacing: typed as `dY,dX` (Enter keeps the last one), or shown
//!   with two points, the second's offset from the first; a second point on
//!   the first is let go.
//!
//! Written through the product command `cad.entities.array` (a grid), one
//! undo step, “Dizi”; says “2 × 3 dizi oluşturuldu: 5 yeni nesne.” and
//! leaves. The command's refusal (a direction with more than one place and
//! no spacing) is said, and the tool waits for another spacing. The ghosts
//! are the copies where they would go: the last spacing's until the first
//! point, then the cursor's.

use kentos_contracts::ArrayLayout;
use kentos_geometry_core::geom::affine::Affine;
use kentos_geometry_core::jsmath::js_round;
use kentos_geometry_core::tools::editing::grid_array_transforms;
use kentos_geometry_core::tools::point_text::js_trim;

use crate::Vec2;
use crate::format::js_number;
use crate::log::Level;
use crate::modify::{Modify, Stages, array_selection};
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Memory};

/// The array tool's id: its command is `tool.array`.
pub const ID: &str = "array";
pub const LABEL: &str = "Dizi";

/// What the tool asks for after the objects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Stage {
    #[default]
    Count,
    Spacing,
}

/// The array tool's stages.
#[derive(Clone, Debug)]
pub struct Array {
    rows: u32,
    cols: u32,
    stage: Stage,
    base: Option<Vec2>,
    /// What the session remembers (the web's `ArrayTool.last`), seen at every event.
    last: Memory,
}

impl Array {
    pub fn tool() -> Modify<Self> {
        let last = Memory::default();
        Modify::with(Self {
            rows: last.array_rows,
            cols: last.array_cols,
            stage: Stage::Count,
            base: None,
            last,
        })
    }

    /// Writes the array with this spacing: none when both are within a
    /// nanometre of zero (the web lets such a point go); the tool leaves once
    /// it is written, and waits for another spacing when it is refused.
    fn build(&mut self, dx: f64, dy: f64, cx: &mut Context<'_>) -> Flow {
        if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
            return Flow::Stay;
        }
        let layout = ArrayLayout::Grid {
            rows: self.rows,
            cols: self.cols,
            dx,
            dy,
        };
        let Some(n) = array_selection(layout, cx) else {
            return Flow::Stay;
        };
        let memory = &mut *cx.memory;
        memory.array_rows = self.rows;
        memory.array_cols = self.cols;
        memory.array_dx = dx;
        memory.array_dy = dy;
        let line = format!(
            "{} × {} dizi oluşturuldu: {n} yeni nesne.",
            self.rows, self.cols
        );
        cx.say(Level::Success, line);
        Flow::Exit
    }

    fn offsets(&self, dx: f64, dy: f64) -> Vec<Affine> {
        // Each offset is one product, the same bits in the core (docs/adr/0008, S5).
        grid_array_transforms(f64::from(self.rows), f64::from(self.cols), dx, dy)
    }
}

/// Two numbers as the web's array tool reads them
/// (`/^(-?\d+(?:\.\d+)?)\s*[,; ]\s*(-?\d+(?:\.\d+)?)$/` on the trimmed text):
/// plain decimals, a comma, a semicolon or a space between.
fn pair(text: &str) -> Option<(f64, f64)> {
    let text = js_trim(text);
    let (first, rest) = number_at(text)?;
    let gap_end = rest
        .find(|c: char| !(c.is_whitespace() || c == ',' || c == ';'))
        .unwrap_or(rest.len());
    let (gap, rest) = rest.split_at(gap_end);
    let marks = gap.chars().filter(|c| *c == ',' || *c == ';').count();
    // One comma or semicolon among spaces, or a space alone (`\s*[,; ]\s*`).
    let parted = marks == 1 || (marks == 0 && gap.contains(' '));
    if !parted {
        return None;
    }
    let (second, rest) = number_at(rest)?;
    rest.is_empty().then_some((first, second))
}

/// `-?\d+(?:\.\d+)?` at the start of `text`: its value and what follows.
fn number_at(text: &str) -> Option<(f64, &str)> {
    let bytes = text.as_bytes();
    let mut at = usize::from(bytes.first() == Some(&b'-'));
    let digits = |from: usize| {
        bytes[from..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count()
    };
    let whole = digits(at);
    if whole == 0 {
        return None;
    }
    at += whole;
    if bytes.get(at) == Some(&b'.') {
        let fraction = digits(at + 1);
        if fraction > 0 {
            at += 1 + fraction;
        }
    }
    let value = text[..at].parse::<f64>().ok()?;
    Some((value, &text[at..]))
}

impl Stages for Array {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        self.last = *cx.memory;
        self.rows = self.last.array_rows;
        self.cols = self.last.array_cols;
        self.stage = Stage::Count;
        self.base = None;
        Flow::Stay
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.last = *cx.memory;
    }

    fn anchor(&self) -> Option<Vec2> {
        self.base
    }

    fn prompt(&self, _n: usize) -> Prompt {
        let l = &self.last;
        let step = match (self.stage, self.base) {
            (Stage::Count, _) => format!(
                "satır ve sütun sayısını yazın, ör. {r},{c} (Enter: {r},{c})",
                r = l.array_rows,
                c = l.array_cols
            ),
            (Stage::Spacing, Some(_)) => "aralık için ikinci noktayı gösterin".to_owned(),
            (Stage::Spacing, None) => format!(
                "sütun ve satır aralığını yazın dY,dX (Enter: {},{}) ya da iki nokta gösterin",
                js_number(l.array_dx),
                js_number(l.array_dy)
            ),
        };
        Prompt::new(LABEL, step)
    }

    fn point(&mut self, p: Vec2, cx: &mut Context<'_>) -> Flow {
        if self.stage != Stage::Spacing {
            return Flow::Stay;
        }
        let Some(base) = self.base else {
            self.base = Some(p);
            return Flow::Stay;
        };
        self.build(p.x - base.x, p.y - base.y, cx)
    }

    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        let (a, b) = pair(text)?;
        if self.stage == Stage::Spacing {
            return Some(self.build(a, b, cx));
        }
        let (r, c) = (js_round(a), js_round(b));
        if r < 1.0 || c < 1.0 || r * c < 2.0 || r * c > 10_000.0 {
            cx.say(Level::Warn, "Satır × sütun 2 ile 10 000 arasında olmalı.");
            return Some(Flow::Stay);
        }
        // Whole numbers from 1 to 10 000: exact as u32.
        self.rows = r as u32;
        self.cols = c as u32;
        self.stage = Stage::Spacing;
        Some(Flow::Stay)
    }

    /// Dizi reads counts and spacings, never a typed point (the web's `input`).
    fn typed_points(&self) -> bool {
        false
    }

    /// Enter: the last rows and columns, then the last spacing.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.stage == Stage::Count {
            self.stage = Stage::Spacing;
            return Flow::Stay;
        }
        self.build(self.last.array_dx, self.last.array_dy, cx)
    }

    fn preview(&self, _hover: Vec2) -> Option<Affine> {
        None
    }

    fn previews(&self, hover: Option<Vec2>) -> Vec<Affine> {
        if self.stage != Stage::Spacing {
            return Vec::new();
        }
        match (self.base, hover) {
            (Some(base), Some(hover)) => self.offsets(hover.x - base.x, hover.y - base.y),
            _ => self.offsets(self.last.array_dx, self.last.array_dy),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The web's pattern: two plain decimals, one comma, semicolon or space
    /// between (spaces around it), nothing else.
    #[test]
    fn two_numbers_read_as_the_web_reads_them() {
        assert_eq!(pair("2,3"), Some((2.0, 3.0)));
        assert_eq!(pair(" 2 ; 3 "), Some((2.0, 3.0)));
        assert_eq!(pair("2 3"), Some((2.0, 3.0)));
        assert_eq!(pair("-12.5, 7.25"), Some((-12.5, 7.25)));
        assert_eq!(pair("2 ,  3"), Some((2.0, 3.0)));
        for text in [
            "2", "2,,3", "2,3,4", "+2,3", "2.,3", ".5,3", "2e1,3", "2\t3", "a,b", "2,3m",
        ] {
            assert_eq!(pair(text), None, "{text}");
        }
    }
}
