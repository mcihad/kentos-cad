//! What a LEADER and a MULTILEADER say, as the converter needs it
//! (docs/adr/0146 §8): a dimension style's leader sizes and arrow block
//! (its table record, or an entity's own changes in ACAD's DSTYLE data),
//! the arrowhead an AutoCAD arrow block's name stands for, and a
//! MULTILEADER's leader lines and note out of its nested sections.

use kentos_contracts::LeaderArrow;

use super::entity::{P3, Unreadable, bad};
use super::lexer::Pair;
use super::strings::Decoder;
use crate::num::{parse_int, parse_real};

/// What a dimension style says of a leader: the arrow size (DIMASZ, 41),
/// the overall scale (DIMSCALE, 40) and the leader's arrow block (DIMLDRBLK,
/// 341, a block record's handle). An entity's own changes say the same.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LeaderStyle {
    pub arrow_size: Option<f64>,
    pub scale: Option<f64>,
    pub arrow_block: Option<u64>,
}

impl LeaderStyle {
    /// `self`'s values, `under`'s where `self` has none.
    pub fn over(self, under: LeaderStyle) -> LeaderStyle {
        LeaderStyle {
            arrow_size: self.arrow_size.or(under.arrow_size),
            scale: self.scale.or(under.scale),
            arrow_block: self.arrow_block.or(under.arrow_block),
        }
    }

    /// The arrow's length in drawing units (DIMASZ × DIMSCALE; a scale of 0
    /// or none is 1), when the style gives one over 0.
    pub fn arrow_length(&self) -> Option<f64> {
        let scale = self.scale.filter(|s| *s > 0.0).unwrap_or(1.0);
        self.arrow_size
            .filter(|a| *a > 0.0)
            .map(|a| a * scale)
            .filter(|l| l.is_finite())
    }
}

fn handle(p: &Pair<'_>) -> Option<u64> {
    u64::from_str_radix(p.text(), 16).ok()
}

/// A dimension style's table record (DIMSTYLE).
pub fn table_style(groups: &[Pair<'_>]) -> LeaderStyle {
    let find = |code: i32| groups.iter().find(|p| p.code == code);
    LeaderStyle {
        arrow_size: find(41).and_then(|p| parse_real(p.text())),
        scale: find(40).and_then(|p| parse_real(p.text())),
        arrow_block: find(341).and_then(handle),
    }
}

/// An entity's own changes to its dimension style, in ACAD's extended data:
/// `1001 ACAD`, `1000 DSTYLE`, `1002 {`, then a variable's code (1070) and
/// its value, … `1002 }`. Only a leader's three are kept.
pub fn own_style(groups: &[Pair<'_>]) -> LeaderStyle {
    let mut s = LeaderStyle::default();
    let Some(start) = groups
        .iter()
        .position(|p| p.code == 1001 && p.text().eq_ignore_ascii_case("ACAD"))
    else {
        return s;
    };
    let rest = &groups[start + 1..];
    let rest = &rest[..rest.iter().position(|p| p.code == 1001).unwrap_or(rest.len())];
    let Some(k) = rest
        .iter()
        .position(|p| p.code == 1000 && p.text().eq_ignore_ascii_case("DSTYLE"))
    else {
        return s;
    };
    let mut code: Option<i64> = None;
    for p in &rest[k + 1..] {
        if p.code == 1002 {
            if p.text() == "}" {
                break;
            }
            continue;
        }
        match code.take() {
            None => code = (p.code == 1070).then(|| parse_int(p.text())).flatten(),
            Some(41) => s.arrow_size = parse_real(p.text()),
            Some(40) => s.scale = parse_real(p.text()),
            Some(341) => s.arrow_block = handle(p),
            Some(_) => {}
        }
    }
    s
}

/// The arrowhead an AutoCAD arrow block's name stands for: none is the
/// filled one. `false` when KentOS has no such arrowhead (it is drawn filled).
pub fn arrow_of_block(name: &str) -> (Option<LeaderArrow>, bool) {
    let n = name.trim().to_uppercase();
    match n.as_str() {
        "" | "_CLOSEDFILLED" => (None, true),
        "_NONE" => (Some(LeaderArrow::None), true),
        "_SMALL" => (Some(LeaderArrow::Dot), true),
        _ if n.starts_with("_OPEN") => (Some(LeaderArrow::Open), true),
        _ if n.starts_with("_DOT") => (Some(LeaderArrow::Dot), true),
        _ => (None, false),
    }
}

/// A MULTILEADER's groups as the converter needs them.
#[derive(Clone, Debug, Default)]
pub struct MLeader {
    /// Each leader line's vertices from its arrowhead, its leader's last
    /// point (where the landing starts) after them.
    pub lines: Vec<Vec<P3>>,
    /// The MTEXT content (MTEXT's notation), when it has one.
    pub text: Option<String>,
    /// Where the content stands (12), its direction (13), its height (41)
    /// and alignment (171: 1 left, 2 centre, 3 right), line spacing (45)
    /// and whether it is drawn over a background (292).
    pub text_at: P3,
    pub text_dir: Option<P3>,
    pub text_height: f64,
    pub alignment: i64,
    pub spacing: f64,
    pub mask: bool,
    /// Its content is a block (296 or 172 = 1).
    pub block: bool,
    /// The arrowhead's length (42, else the context's 140) and block (342).
    pub arrow_size: f64,
    pub arrow_block: Option<u64>,
    /// Its leader lines are splines (170 = 2).
    pub spline: bool,
}

/// Where the reading is among a MULTILEADER's nested sections.
#[derive(Clone, Copy, PartialEq)]
enum In {
    Entity,
    Context,
    Leader,
    Line,
    After,
}

/// A MULTILEADER from its groups: the context (300 `CONTEXT_DATA{` … 301
/// `}`) with its leaders (302 `LEADER{` … 303 `}`) and their lines (304
/// `LEADER_LINE{` … 305 `}`), then the entity's own properties.
pub fn mleader(list: &[Pair<'_>], dec: Decoder) -> Result<MLeader, Unreadable> {
    let mut m = MLeader {
        alignment: 1,
        spacing: 1.0,
        ..MLeader::default()
    };
    let mut at = In::Entity;
    // The leader being read: its last point, and its lines' vertices.
    let mut last: Option<P3> = None;
    let mut lines: Vec<Vec<P3>> = Vec::new();
    // The context's first 290: whether its content is an MTEXT.
    let mut has_text: Option<bool> = None;
    let mut context_arrow = 0.0;
    for p in list {
        let num = || {
            parse_real(p.text()).ok_or_else(|| {
                bad(&format!(
                    "sayı okunamadı (grup {}, satır {})",
                    p.code, p.line
                ))
            })
        };
        let text = p.text();
        match (at, p.code) {
            (In::Entity, 300) if text.eq_ignore_ascii_case("CONTEXT_DATA{") => at = In::Context,
            (In::Context, 302) if text.eq_ignore_ascii_case("LEADER{") => {
                at = In::Leader;
                last = None;
                lines = Vec::new();
            }
            (In::Context, 301) => at = In::After,
            (In::Context, 41) => m.text_height = num()?,
            (In::Context, 140) => context_arrow = num()?,
            (In::Context, 290) if has_text.is_none() => {
                has_text = Some(parse_int(text).unwrap_or(0) != 0);
            }
            (In::Context, 304) => m.text = Some(dec.string(p.value)),
            (In::Context, 12) => m.text_at[0] = num()?,
            (In::Context, 22) => m.text_at[1] = num()?,
            (In::Context, 32) => m.text_at[2] = num()?,
            (In::Context, 13) => m.text_dir = Some([num()?, 0.0, 0.0]),
            (In::Context, 23) => {
                if let Some(d) = &mut m.text_dir {
                    d[1] = num()?;
                }
            }
            (In::Context, 45) => m.spacing = num()?,
            (In::Context, 171) => m.alignment = parse_int(text).unwrap_or(1),
            (In::Context, 292) => m.mask = parse_int(text).unwrap_or(0) != 0,
            (In::Context, 296) => m.block |= parse_int(text).unwrap_or(0) != 0,
            (In::Leader, 304) if text.eq_ignore_ascii_case("LEADER_LINE{") => {
                at = In::Line;
                lines.push(Vec::new());
            }
            (In::Leader, 303) => {
                at = In::Context;
                for mut line in std::mem::take(&mut lines) {
                    line.extend(last);
                    m.lines.push(line);
                }
            }
            (In::Leader, 10) => last = Some([num()?, 0.0, 0.0]),
            (In::Leader, 20) => {
                if let Some(q) = &mut last {
                    q[1] = num()?;
                }
            }
            (In::Leader, 30) => {
                if let Some(q) = &mut last {
                    q[2] = num()?;
                }
            }
            (In::Line, 305) => at = In::Leader,
            (In::Line, 10) => {
                if let Some(line) = lines.last_mut() {
                    line.push([num()?, 0.0, 0.0]);
                }
            }
            (In::Line, 20 | 30) => {
                if let Some(q) = lines.last_mut().and_then(|l| l.last_mut()) {
                    q[if p.code == 20 { 1 } else { 2 }] = num()?;
                }
            }
            (In::After, 170) => m.spline = parse_int(text) == Some(2),
            (In::After, 342) => m.arrow_block = handle(p).filter(|h| *h != 0),
            (In::After, 42) => m.arrow_size = num()?,
            (In::After, 172) => m.block |= parse_int(text) == Some(1),
            _ => {}
        }
    }
    if !(m.arrow_size > 0.0) {
        m.arrow_size = context_arrow;
    }
    // A content that is not an MTEXT, or an MTEXT of nothing, is none.
    if has_text == Some(false) || m.text.as_deref().is_some_and(|t| t.trim().is_empty()) {
        m.text = None;
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dxf::lexer::Lexer;
    use crate::text::Encoding;

    fn groups(text: &str) -> Vec<Pair<'_>> {
        let mut lex = Lexer::new(text.as_bytes());
        let mut out = Vec::new();
        while let Ok(Some(p)) = lex.next() {
            out.push(p);
        }
        out
    }

    #[test]
    fn an_entity_s_own_style_comes_from_acad_s_dstyle_list() {
        let g = groups(
            "1001\nKENTOS\n1002\n{\n1000\nlabel\n1000\nP\n1002\n}\n1001\nACAD\n1000\nDSTYLE\n1002\n{\n1070\n77\n1070\n1\n1070\n41\n1040\n3.0\n1070\n341\n1005\n1F\n1002\n}\n",
        );
        assert_eq!(
            own_style(&g),
            LeaderStyle {
                arrow_size: Some(3.0),
                scale: None,
                arrow_block: Some(0x1F),
            }
        );
        let style = LeaderStyle {
            arrow_size: Some(1.8),
            scale: Some(2.0),
            arrow_block: None,
        };
        assert_eq!(own_style(&g).over(style).arrow_length(), Some(3.0 * 2.0));
        assert_eq!(style.arrow_length(), Some(3.6));
        assert_eq!(LeaderStyle::default().arrow_length(), None);
    }

    #[test]
    fn arrow_blocks_name_the_arrowheads_kentos_has() {
        assert_eq!(arrow_of_block(""), (None, true));
        assert_eq!(arrow_of_block("_ClosedFilled"), (None, true));
        assert_eq!(arrow_of_block("_Open30"), (Some(LeaderArrow::Open), true));
        assert_eq!(arrow_of_block("_DotSmall"), (Some(LeaderArrow::Dot), true));
        assert_eq!(arrow_of_block("_none"), (Some(LeaderArrow::None), true));
        assert_eq!(arrow_of_block("_ArchTick"), (None, false));
    }

    #[test]
    fn a_multileader_s_sections_give_its_lines_and_note() {
        let g = groups(
            "300\nCONTEXT_DATA{\n40\n1.0\n10\n0\n20\n0\n30\n0\n41\n2.0\n140\n1.25\n290\n1\n304\nAda 101\\PParsel 5\n12\n109\n22\n7\n32\n0\n13\n1\n23\n0\n33\n0\n171\n1\n292\n1\n296\n0\n302\nLEADER{\n290\n1\n10\n106\n20\n6\n30\n0\n11\n1\n21\n0\n31\n0\n40\n2.5\n304\nLEADER_LINE{\n10\n100\n20\n0\n30\n0\n10\n104\n20\n4\n30\n0\n305\n}\n304\nLEADER_LINE{\n10\n110\n20\n0\n30\n0\n305\n}\n303\n}\n301\n}\n170\n1\n342\n1F\n42\n0.0\n172\n2\n",
        );
        let m = mleader(&g, Decoder { enc: Encoding::Utf8 }).ok().expect("reads");
        assert_eq!(
            m.lines,
            vec![
                vec![[100.0, 0.0, 0.0], [104.0, 4.0, 0.0], [106.0, 6.0, 0.0]],
                vec![[110.0, 0.0, 0.0], [106.0, 6.0, 0.0]],
            ]
        );
        assert_eq!(m.text.as_deref(), Some("Ada 101\\PParsel 5"));
        assert_eq!((m.text_at, m.text_dir), ([109.0, 7.0, 0.0], Some([1.0, 0.0, 0.0])));
        assert_eq!((m.text_height, m.alignment, m.mask, m.block), (2.0, 1, true, false));
        // No arrow size of its own: the context's.
        assert_eq!((m.arrow_size, m.arrow_block, m.spline), (1.25, Some(0x1F), false));
    }
}
