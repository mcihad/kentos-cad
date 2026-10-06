//! A HATCH's pattern as KentOS holds it (docs/adr/0186 §9). A gradient by
//! its name, angle (460) and two colours, the first the hatch's own; a
//! user-defined pattern (76 = 0) of one whole family as lines, of two at a
//! right angle as crossed lines; any other by its name, angle (52) and
//! scale (41), its families unturned and unscaled as the pattern defines
//! them (the file holds them turned and scaled); a pattern without
//! families from the library by its name. What is approximated is said.

use kentos_contracts::{
    GradientShape, HatchGradient, HatchPattern, HatchPatternType, MAX_PATTERN_DASHES,
    MAX_PATTERN_LINES, MAX_PATTERN_NAME, MAX_PATTERN_SIZE, PatternLine as Family,
};
use kentos_geometry_core::geom::hatch_pattern::library_pattern;

use crate::dxf::aci;
use crate::dxf::hatch::{GradientColour, Hatch, PatternLine};
use crate::geom::{Tf, v};
use crate::math::{atan2, deg, sin_cos_deg};

/// What a HATCH's pattern became.
pub struct Taken {
    pub pattern: HatchPattern,
    /// The hatch's colour, where a gradient's first colour is not its own.
    pub colour: Option<String>,
    pub notes: Vec<String>,
}

/// An angle in degrees from 0 up to 360.
pub fn turn(a: f64) -> f64 {
    let t = a % 360.0;
    let t = if t < 0.0 { t + 360.0 } else { t };
    if t >= 360.0 || t == 0.0 { 0.0 } else { t }
}

/// The 1/8 inch the ANSI patterns' lines are apart, where a file names a
/// pattern KentOS does not know and gives none of its lines.
const EIGHTH: f64 = 3.175;

/// How the hatch's object coordinates sit in the drawing: its scale, turn
/// (degrees) and whether mirrored; when `m` does not keep shapes, the
/// nearest such (`false` in the last place).
fn similarity(m: Tf) -> (f64, f64, bool, bool) {
    match m.similarity() {
        Some(s) => (s.scale, deg(s.angle), s.mirror, true),
        None => (
            m.det().abs().sqrt(),
            deg(atan2(m.b, m.a)),
            m.det() < 0.0,
            false,
        ),
    }
}

/// The distance between a family's lines (its offset across them).
fn spacing_of(l: &PatternLine) -> f64 {
    let (s, c) = sin_cos_deg(l.angle);
    (l.offset[0] * -s + l.offset[1] * c).abs()
}

fn fits(x: f64) -> bool {
    x.is_finite() && x.abs() <= MAX_PATTERN_SIZE
}

/// A family reflected in its pattern's x axis (the core's `reflected`).
fn reflected(f: Family) -> Family {
    Family {
        angle: turn(-f.angle),
        origin: [f.origin[0], -f.origin[1]],
        offset: [f.offset[0], -f.offset[1]],
        dashes: f.dashes,
    }
}

/// A colour of a gradient as the app names it: an ACI colour whose true
/// colour is its own by its index (7 the ink), else the true colour.
fn colour_name(c: GradientColour) -> Option<String> {
    let index = c
        .aci
        .and_then(|n| u8::try_from(n).ok())
        .filter(|n| (1..=255).contains(n));
    match (index, c.rgb) {
        (Some(n), Some(v)) => {
            let [r, g, b] = aci::rgb(n);
            let own = (i64::from(r) << 16) | (i64::from(g) << 8) | i64::from(b);
            Some(if own == v & 0xFF_FFFF {
                aci::color(n)
            } else {
                aci::true_color(v)
            })
        }
        (None, Some(v)) => Some(aci::true_color(v)),
        (Some(n), None) => Some(aci::color(n)),
        (None, None) => None,
    }
}

/// A colour as `#RRGGBB` (a gradient's second colour): the ink as white,
/// the ACI table's colour.
fn hex(name: &str) -> String {
    if name.starts_with('#') {
        name.to_owned()
    } else {
        "#FFFFFF".to_owned()
    }
}

fn gradient(m: Tf, h: &Hatch, own: Option<&str>, t: &mut Taken) {
    let name = h.gradient_name.trim().to_uppercase();
    let (shape, inverted, near) = match name.as_str() {
        "" | "LINEAR" => (GradientShape::Linear, false, false),
        "CYLINDER" => (GradientShape::Cylinder, false, false),
        "INVCYLINDER" => (GradientShape::Cylinder, true, false),
        "SPHERICAL" => (GradientShape::Spherical, false, false),
        "INVSPHERICAL" => (GradientShape::Spherical, true, false),
        "HEMISPHERICAL" => (GradientShape::Spherical, false, true),
        "INVHEMISPHERICAL" => (GradientShape::Spherical, true, true),
        "CURVED" => (GradientShape::Linear, false, true),
        "INVCURVED" => (GradientShape::Linear, true, true),
        _ => (GradientShape::Linear, false, true),
    };
    if near {
        let word = match shape {
            GradientShape::Linear => "doğrusal",
            GradientShape::Cylinder => "silindir",
            GradientShape::Spherical => "küre",
        };
        t.notes.push(format!(
            "“{}” degradesi en yakın biçimle ({word}{}) alındı",
            h.gradient_name.trim(),
            if inverted { ", ters" } else { "" }
        ));
    }
    if h.gradient_shift != 0.0 {
        t.notes
            .push("degradenin kaydırması alınmadı; ortalanmış olarak alındı".to_owned());
    }
    let named: Vec<String> = h.colours.iter().filter_map(|c| colour_name(*c)).collect();
    if h.one_colour {
        t.notes.push(
            "tek renkli degrade, dosyadaki ikinci rengiyle iki renkli olarak alındı".to_owned(),
        );
    }
    let color2 = match named.get(1) {
        Some(c) => hex(c),
        None => {
            t.notes
                .push("degradenin ikinci rengi dosyada yok; beyaz alındı".to_owned());
            "#FFFFFF".to_owned()
        }
    };
    if let Some(first) = named.first()
        && own != Some(first.as_str())
    {
        t.colour = Some(first.clone());
    }
    let a = deg(h.gradient_angle);
    let angle = if m.is_identity() {
        turn(a)
    } else {
        let (s, c) = sin_cos_deg(a);
        let d = m.linear(v(c, s));
        turn(deg(atan2(d.y, d.x)))
    };
    t.pattern = HatchPattern {
        kind: HatchPatternType::Gradient,
        angle: if angle.is_finite() { angle } else { 0.0 },
        spacing: 1.0,
        name: None,
        scale: None,
        lines: None,
        gradient: Some(HatchGradient {
            shape,
            inverted,
            color2,
        }),
    };
}

/// A user-defined pattern of one whole family as lines, of two whole ones
/// at a right angle and the same spacing as crossed lines.
fn user_lines(m: Tf, h: &Hatch) -> Option<HatchPattern> {
    let lines: Vec<_> = h.lines.iter().filter(|l| spacing_of(l) > 0.0).collect();
    if lines.iter().any(|l| !l.dashes.is_empty()) {
        return None;
    }
    let (kind, angle, spacing) = match lines.as_slice() {
        [one] => (HatchPatternType::Lines, one.angle, spacing_of(one)),
        [a, b]
            if ((a.angle - b.angle).rem_euclid(180.0) - 90.0).abs() < 1e-6
                && (spacing_of(a) - spacing_of(b)).abs() <= 1e-9 * spacing_of(a) =>
        {
            (HatchPatternType::Cross, a.angle, spacing_of(a))
        }
        _ => return None,
    };
    // The lines' direction in the drawing; they repeat every 180°.
    let world = if m.is_identity() {
        angle
    } else {
        let (s, c) = sin_cos_deg(angle);
        let d = m.linear(v(c, s));
        deg(atan2(d.y, d.x))
    };
    let spacing = if m.is_identity() {
        spacing
    } else {
        spacing * m.det().abs().sqrt()
    };
    Some(HatchPattern::user(kind, world.rem_euclid(180.0), spacing))
}

/// A pattern's families as it defines them: unturned by its angle `alpha`,
/// unscaled by its scale `s`.
fn families(h: &Hatch, alpha: f64, s: f64, notes: &mut Vec<String>) -> Vec<Family> {
    let (sa, ca) = sin_cos_deg(alpha);
    let mut out = Vec::new();
    let mut dropped = 0usize;
    for l in &h.lines {
        if out.len() == MAX_PATTERN_LINES {
            notes.push(format!(
                "“{}” deseninin ilk {MAX_PATTERN_LINES} çizgi ailesi alındı ({} aile var)",
                h.name,
                h.lines.len()
            ));
            break;
        }
        let (sl, cl) = sin_cos_deg(l.angle);
        let origin = [
            (ca * l.base[0] + sa * l.base[1]) / s,
            (-sa * l.base[0] + ca * l.base[1]) / s,
        ];
        let offset = [
            (cl * l.offset[0] + sl * l.offset[1]) / s,
            (-sl * l.offset[0] + cl * l.offset[1]) / s,
        ];
        let mut dashes: Vec<f64> = l.dashes.iter().map(|d| d / s).collect();
        if dashes.len() > MAX_PATTERN_DASHES {
            notes.push(format!(
                "“{}” deseninin bir ailesinin ilk {MAX_PATTERN_DASHES} kesiği alındı ({} kesik var)",
                h.name,
                dashes.len()
            ));
            dashes.truncate(MAX_PATTERN_DASHES);
        }
        let angle = turn(l.angle - alpha);
        let numbers_fit = [angle, origin[0], origin[1], offset[0], offset[1]]
            .into_iter()
            .chain(dashes.iter().copied())
            .all(fits);
        let some_length = dashes.is_empty() || dashes.iter().any(|d| *d != 0.0);
        if !numbers_fit || offset[1] == 0.0 || !some_length {
            dropped += 1;
            continue;
        }
        out.push(Family {
            angle,
            origin,
            offset,
            dashes,
        });
    }
    if dropped > 0 {
        notes.push(format!(
            "“{}” deseninin {dropped} çizgi ailesi geçersiz (çizgileri arası 0, sayıları sonsuz ya da kesiklerinin hepsi 0); alınmadı",
            h.name
        ));
    }
    out
}

/// A HATCH's pattern in the drawing whose object coordinates `m` places;
/// `own` the hatch's colour as it is drawn (its own, else its layer's).
pub fn pattern_of(m: Tf, h: &Hatch, own: Option<&str>) -> Taken {
    let mut t = Taken {
        pattern: HatchPattern::user(HatchPatternType::Solid, 0.0, 1.0),
        colour: None,
        notes: Vec::new(),
    };
    if h.gradient {
        gradient(m, h, own, &mut t);
        return t;
    }
    if h.solid || h.name.eq_ignore_ascii_case("SOLID") {
        return t;
    }
    let user = match h.kind {
        Some(k) => k == 0,
        None => {
            let n = h.name.trim();
            n.is_empty() || n.eq_ignore_ascii_case("_USER") || n.eq_ignore_ascii_case("USER")
        }
    };
    if user && let Some(p) = user_lines(m, h) {
        t.pattern = p;
        return t;
    }
    let (k, phi, mirror, keeps) = similarity(m);
    if !keeps {
        t.notes.push(
            "taramanın deseni eşit ölçeklenmeyen bir blok yerleştirmesinde; yaklaşık alındı"
                .to_owned(),
        );
    }
    let alpha = if h.angle.is_finite() { h.angle } else { 0.0 };
    let s = if h.scale.is_finite() && h.scale > 0.0 {
        h.scale
    } else {
        1.0
    };
    let angle = turn(if mirror { phi - alpha } else { phi + alpha });
    let mut name: String = h.name.trim().chars().take(MAX_PATTERN_NAME).collect();
    if name.is_empty() {
        name = "_USER".to_owned();
    }
    let mut lines = families(h, alpha, s, &mut t.notes);
    if lines.is_empty()
        && let Some(lib) = library_pattern(&name)
    {
        // When its families were all dropped, that was said; the library's stand in.
        if h.lines.is_empty() {
            t.notes.push(format!(
                "“{name}” deseninin çizgileri dosyada yok; kitaplıktaki tanımıyla alındı"
            ));
        }
        name = lib.name.to_owned();
        lines = lib
            .lines
            .iter()
            .map(|l| Family {
                angle: l.angle,
                origin: l.origin,
                offset: l.offset,
                dashes: l.dashes.clone().unwrap_or_default(),
            })
            .collect();
    }
    if lines.is_empty() {
        // Nothing to draw it with: 45° lines an eighth of an inch apart, at the pattern's angle and scale.
        t.notes.push(format!(
            "“{}” deseninin çizgileri dosyada yok; 45° çizgilerle yaklaşık alındı",
            h.name
        ));
        let spacing = EIGHTH * s * k;
        let lines_angle = turn(if mirror {
            phi - alpha - 45.0
        } else {
            phi + alpha + 45.0
        });
        t.pattern = HatchPattern::user(
            HatchPatternType::Lines,
            lines_angle.rem_euclid(180.0),
            spacing,
        );
        return t;
    }
    if mirror {
        lines = lines.into_iter().map(reflected).collect();
    }
    t.pattern = HatchPattern {
        kind: HatchPatternType::Pattern,
        angle,
        spacing: 1.0,
        name: Some(name),
        scale: Some(s * k),
        lines: Some(lines),
        gradient: None,
    };
    t
}

/// Two numbers the same to a billionth of the larger (and of 1).
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

/// Two angles (degrees) the same direction to a billionth of a degree.
fn same_turn(a: f64, b: f64) -> bool {
    let d = turn(a - b);
    d <= 1e-9 || 360.0 - d <= 1e-9
}

/// A pattern as DXF writes it: an inverted linear gradient is the linear
/// one turned half round (DXF names no inverted linear gradient).
fn as_written(p: &HatchPattern) -> HatchPattern {
    let mut out = p.clone();
    if let Some(g) = &mut out.gradient
        && g.shape == GradientShape::Linear
        && g.inverted
    {
        g.inverted = false;
        out.angle = turn(p.angle + 180.0);
    }
    out
}

/// Whether a pattern read from the groups says what KentOS's data holds.
fn agrees(read: &HatchPattern, kept: &HatchPattern) -> bool {
    let (a, b) = (as_written(read), as_written(kept));
    if a.kind != b.kind || !same_turn(a.angle, b.angle) {
        return false;
    }
    match a.kind {
        HatchPatternType::Pattern => {
            let (la, lb) = (
                a.lines.as_deref().unwrap_or_default(),
                b.lines.as_deref().unwrap_or_default(),
            );
            a.name == b.name
                && close(a.scale.unwrap_or(1.0), b.scale.unwrap_or(1.0))
                && la.len() == lb.len()
                && la.iter().zip(lb).all(|(x, y)| {
                    same_turn(x.angle, y.angle)
                        && close(x.origin[0], y.origin[0])
                        && close(x.origin[1], y.origin[1])
                        && close(x.offset[0], y.offset[0])
                        && close(x.offset[1], y.offset[1])
                        && x.dashes.len() == y.dashes.len()
                        && x.dashes.iter().zip(&y.dashes).all(|(p, q)| close(*p, *q))
                })
        }
        HatchPatternType::Gradient => a.gradient == b.gradient,
        _ => false,
    }
}

/// The pattern KentOS's data holds (`kept`, the contract's JSON) while the
/// file's groups still say it: an edit in another program wins.
pub fn exact(read: HatchPattern, kept: Option<&str>) -> HatchPattern {
    let Some(kept) = kept.and_then(|j| serde_json::from_str::<HatchPattern>(j).ok()) else {
        return read;
    };
    if kept.problem().is_none() && agrees(&read, &kept) {
        kept
    } else {
        read
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hatch(
        name: &str,
        kind: Option<i64>,
        angle: f64,
        scale: f64,
        lines: Vec<PatternLine>,
    ) -> Hatch {
        Hatch {
            elevation: 0.0,
            name: name.into(),
            solid: false,
            assoc: false,
            style: 0,
            kind,
            angle,
            scale,
            double: false,
            lines,
            paths: Vec::new(),
            gradient: false,
            gradient_name: String::new(),
            gradient_angle: 0.0,
            gradient_shift: 0.0,
            one_colour: false,
            colours: Vec::new(),
        }
    }

    fn line(angle: f64, base: [f64; 2], offset: [f64; 2], dashes: &[f64]) -> PatternLine {
        PatternLine {
            angle,
            base,
            offset,
            dashes: dashes.to_vec(),
        }
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn a_pattern_turned_and_scaled_in_the_file_is_its_definition() {
        // A family at 45° through (4.49, 0), lines 6.35 apart, dashes 3.175 and −1.5875: in the file turned 30° and twice as large.
        let (s30, c30) = sin_cos_deg(30.0);
        let (s75, c75) = sin_cos_deg(75.0);
        let base = [2.0 * 4.49 * c30, 2.0 * 4.49 * s30];
        let off = [-2.0 * 6.35 * s75, 2.0 * 6.35 * c75];
        let h = hatch(
            "ANSI33",
            Some(1),
            30.0,
            2.0,
            vec![line(75.0, base, off, &[6.35, -3.175])],
        );
        let t = pattern_of(Tf::IDENTITY, &h, None);
        let p = &t.pattern;
        assert_eq!(p.kind, HatchPatternType::Pattern);
        assert_eq!(
            (p.name.as_deref(), p.angle, p.scale),
            (Some("ANSI33"), 30.0, Some(2.0))
        );
        let f = &p.lines.as_ref().expect("families")[0];
        assert!(near(f.angle, 45.0), "{f:?}");
        assert!(near(f.origin[0], 4.49) && near(f.origin[1], 0.0), "{f:?}");
        assert!(near(f.offset[0], 0.0) && near(f.offset[1], 6.35), "{f:?}");
        assert_eq!(f.dashes, vec![3.175, -1.5875]);
        assert!(t.notes.is_empty(), "{:?}", t.notes);
    }

    #[test]
    fn user_lines_and_a_mirrored_pattern() {
        let one = hatch(
            "_USER",
            Some(0),
            30.0,
            2.0,
            vec![line(30.0, [0.0; 2], [-1.0, 1.7320508075688772], &[])],
        );
        let p = pattern_of(Tf::IDENTITY, &one, None).pattern;
        assert_eq!((p.kind, p.angle), (HatchPatternType::Lines, 30.0));
        assert!(near(p.spacing, 2.0));
        // In a mirrored insert the definition is reflected and the turn runs the other way.
        let m = Tf::scale(-1.0, 1.0);
        let h = hatch(
            "BRICK",
            Some(1),
            10.0,
            1.0,
            vec![line(90.0, [1.0, 2.0], [-3.0, 0.0], &[3.0, -3.0])],
        );
        let p = pattern_of(m, &h, None).pattern;
        assert_eq!((p.angle, p.scale), (170.0, Some(1.0)));
        let f = &p.lines.expect("families")[0];
        assert!(near(f.angle, 280.0), "{f:?}");
    }

    #[test]
    fn a_gradient_its_shape_colours_and_what_is_said() {
        let mut h = hatch("SOLID", Some(1), 0.0, 1.0, Vec::new());
        h.solid = true;
        h.gradient = true;
        h.gradient_name = "INVCYLINDER".into();
        h.gradient_angle = std::f64::consts::FRAC_PI_2;
        h.colours = vec![
            GradientColour {
                aci: Some(5),
                rgb: Some(0x0000FF),
            },
            GradientColour {
                aci: Some(2),
                rgb: Some(0x7FB2E5),
            },
        ];
        let t = pattern_of(Tf::IDENTITY, &h, None);
        let g = t.pattern.gradient.as_ref().expect("gradient");
        assert_eq!(
            (g.shape, g.inverted, g.color2.as_str()),
            (GradientShape::Cylinder, true, "#7FB2E5")
        );
        assert!(near(t.pattern.angle, 90.0), "{}", t.pattern.angle);
        assert_eq!(t.colour.as_deref(), Some("#0000FF"));
        // Its own colour already the first: kept as it is (by layer stays by layer).
        assert_eq!(pattern_of(Tf::IDENTITY, &h, Some("#0000FF")).colour, None);
        h.gradient_name = "HEMISPHERICAL".into();
        let t = pattern_of(Tf::IDENTITY, &h, None);
        assert_eq!(
            t.pattern.gradient.expect("gradient").shape,
            GradientShape::Spherical
        );
        assert!(
            t.notes[0].contains("en yakın biçimle (küre)"),
            "{:?}",
            t.notes
        );
    }

    #[test]
    fn kentos_data_wins_while_the_groups_say_it() {
        let kept = HatchPattern {
            kind: HatchPatternType::Gradient,
            angle: 30.0,
            spacing: 1.0,
            name: None,
            scale: None,
            lines: None,
            gradient: Some(HatchGradient {
                shape: GradientShape::Linear,
                inverted: true,
                color2: "#FFFFFF".into(),
            }),
        };
        let json = serde_json::to_string(&kept).expect("json");
        // DXF has no inverted linear gradient: it reads the linear one half round.
        let mut read = kept.clone();
        read.angle = 210.00000000000003;
        read.gradient.as_mut().expect("gradient").inverted = false;
        assert_eq!(exact(read.clone(), Some(&json)), kept);
        read.angle = 211.0;
        assert_eq!(exact(read.clone(), Some(&json)), read);
    }
}
