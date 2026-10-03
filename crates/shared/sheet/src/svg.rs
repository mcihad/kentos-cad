//! A display list as SVG (design §9). The user unit is the micrometre, so
//! every coordinate is a whole number; angles are written with at most three
//! decimals and opacities with two, by the same rule everywhere: the same
//! list gives the same bytes on every platform. A map's content is the
//! host's (a picture it gives); without one a light grey “harita” box stands
//! in. Typefaces are named, not embedded (embedding waits for the owner's
//! decision, design “Açık sorular” 2); a character a text's face lacks is
//! named with the drawing face that draws it (`text::runs`, a `<tspan>`),
//! one no face has is the display list's “?” (design §6).

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::display::{ArcSeg, DisplayList, Prim, Seg, TextPrim};
use crate::model::{ItemId, yes};
use crate::style::{LineCap, LineJoin, Stroke, TextStyle, alpha};
use crate::text;
use crate::units::{Mdeg, PointUm, RectUm, sin_cos, thousandths_text};

/// A picture for a map frame's content, or for an asset: a data URL or a path the SVG's reader can open.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct MapImage {
    pub item: ItemId,
    pub href: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct AssetHref {
    pub sha256: String,
    pub href: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SvgOptions {
    #[serde(default)]
    pub maps: Vec<MapImage>,
    #[serde(default)]
    pub assets: Vec<AssetHref>,
    /// A light grey box with “harita” where a map has no picture.
    #[serde(default = "yes")]
    pub placeholders: bool,
    /// The document's `<title>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub title: Option<String>,
}

impl Default for SvgOptions {
    fn default() -> Self {
        SvgOptions {
            maps: Vec::new(),
            assets: Vec::new(),
            placeholders: true,
            title: None,
        }
    }
}

/// The CSS family of a `DRAWING_FONTS` id, with its fallback.
pub fn css_family(font: &str) -> &'static str {
    match font {
        "arimo" => "Arimo, Arial, sans-serif",
        "overpass" => "Overpass, sans-serif",
        "quicksand" => "Quicksand, sans-serif",
        "architects-daughter" => "'Architects Daughter', sans-serif",
        "courier-prime" => "'Courier Prime', 'Courier New', monospace",
        "plex-mono" => "'IBM Plex Mono', monospace",
        _ => "Barlow, sans-serif",
    }
}

/// A text's content: its face's pieces as they are, another face's piece in a `<tspan>` naming
/// that face (`text::runs`), so a reader draws every letter from the drawing's faces.
fn text_content(t: &TextPrim) -> String {
    let face = text::face(&TextStyle {
        font: t.font.clone(),
        size: t.size,
        weight: t.weight,
        italic: t.italic,
        color: String::new(),
    });
    let Some(face) = face else {
        return esc(&t.text);
    };
    let mut out = String::with_capacity(t.text.len() + 16);
    for r in text::runs(face, &t.text) {
        if std::ptr::eq(r.face, face) {
            out.push_str(&esc(&r.text));
        } else {
            let italic = if r.face.italic != t.italic {
                if r.face.italic {
                    r#" font-style="italic""#
                } else {
                    r#" font-style="normal""#
                }
            } else {
                ""
            };
            let _ = write!(
                out,
                r#"<tspan font-family="{}" font-weight="{}"{italic}>{}</tspan>"#,
                css_family(&r.face.font),
                r.face.weight,
                esc(&r.text)
            );
        }
    }
    out
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c if (c as u32) < 0x20 && c != '\t' => {}
            c => out.push(c),
        }
    }
    out
}

/// `#rrggbb` and, for `#rrggbbaa`, the opacity as its own attribute.
fn paint(attr: &str, color: &str) -> String {
    let a = alpha(color);
    let rgb = if color.len() == 9 {
        color.get(..7).unwrap_or(color)
    } else {
        color
    };
    if a == 255 {
        format!(r#" {attr}="{rgb}""#)
    } else {
        format!(
            r#" {attr}="{rgb}" {attr}-opacity="{}""#,
            unit_text(u32::from(a), 255)
        )
    }
}

/// `n / d` with at most two decimals (opacity).
fn unit_text(n: u32, d: u32) -> String {
    let hundredths = (u64::from(n) * 100 + u64::from(d) / 2) / u64::from(d.max(1));
    if hundredths >= 100 {
        "1".to_owned()
    } else if hundredths % 10 == 0 {
        format!("0.{}", hundredths / 10)
    } else {
        format!("0.{hundredths:02}")
    }
}

fn stroke_attrs(s: &Stroke) -> String {
    let mut out = paint("stroke", &s.color);
    let _ = write!(out, r#" stroke-width="{}""#, s.width.max(1));
    if !s.dash.is_empty() {
        let d: Vec<String> = s.dash.iter().map(|x| x.to_string()).collect();
        let _ = write!(out, r#" stroke-dasharray="{}""#, d.join(" "));
    }
    match s.cap {
        LineCap::Butt => {}
        LineCap::Round => out.push_str(r#" stroke-linecap="round""#),
        LineCap::Square => out.push_str(r#" stroke-linecap="square""#),
    }
    match s.join {
        LineJoin::Miter => {}
        LineJoin::Round => out.push_str(r#" stroke-linejoin="round""#),
        LineJoin::Bevel => out.push_str(r#" stroke-linejoin="bevel""#),
    }
    out
}

fn fill_stroke(fill: Option<&str>, stroke: Option<&Stroke>) -> String {
    let mut out = match fill {
        Some(f) => paint("fill", f),
        None => r#" fill="none""#.to_owned(),
    };
    if let Some(s) = stroke {
        out.push_str(&stroke_attrs(s));
    }
    out
}

fn rotation(a: Mdeg, r: &RectUm) -> String {
    if a.rem_euclid(360_000) == 0 {
        return String::new();
    }
    let c2 = [
        2 * i64::from(r.left) + i64::from(r.width),
        2 * i64::from(r.top) + i64::from(r.height),
    ];
    // The centre on a half micrometre is written exactly.
    let half = |v: i64| {
        if v % 2 == 0 {
            (v / 2).to_string()
        } else {
            format!("{}.5", v.div_euclid(2))
        }
    };
    format!(
        r#" transform="rotate({} {} {})""#,
        thousandths_text(i64::from(a)),
        half(c2[0]),
        half(c2[1])
    )
}

fn point_rotation(a: Mdeg, p: PointUm) -> String {
    if a.rem_euclid(360_000) == 0 {
        return String::new();
    }
    format!(
        r#" transform="rotate({} {} {})""#,
        thousandths_text(i64::from(a)),
        p[0],
        p[1]
    )
}

fn arc_point(a: &ArcSeg, angle: i64) -> PointUm {
    let (s, c) = sin_cos(crate::units::norm_mdeg(angle));
    let (rs, rc) = sin_cos(a.rotation);
    let (x, y) = (f64::from(a.radius[0]) * c, f64::from(a.radius[1]) * s);
    [
        crate::units::round_um(f64::from(a.center[0]) + x * rc - y * rs),
        crate::units::round_um(f64::from(a.center[1]) + x * rs + y * rc),
    ]
}

fn path_d(segs: &[Seg]) -> String {
    let mut d = String::new();
    let mut cur: Option<PointUm> = None;
    for s in segs {
        match s {
            Seg::M(p) => {
                let _ = write!(d, "M{} {}", p[0], p[1]);
                cur = Some(*p);
            }
            Seg::L(p) => {
                let _ = write!(d, "L{} {}", p[0], p[1]);
                cur = Some(*p);
            }
            Seg::A(a) => {
                let start = arc_point(a, i64::from(a.start));
                match cur {
                    None => {
                        let _ = write!(d, "M{} {}", start[0], start[1]);
                    }
                    Some(c) if c != start => {
                        let _ = write!(d, "L{} {}", start[0], start[1]);
                    }
                    _ => {}
                }
                // A full turn is two halves (SVG cannot draw an arc back to where it starts).
                let total = i64::from(a.sweep).clamp(-360_000, 360_000);
                let parts = if total.abs() > 180_000 { 2 } else { 1 };
                let rot = thousandths_text(i64::from(a.rotation.rem_euclid(360_000)));
                for k in 1..=parts {
                    let end = arc_point(a, i64::from(a.start) + total * k / parts);
                    let large = u8::from((total / parts).abs() > 180_000);
                    let sweep = u8::from(total > 0);
                    let _ = write!(
                        d,
                        "A{} {} {} {} {} {} {}",
                        a.radius[0], a.radius[1], rot, large, sweep, end[0], end[1]
                    );
                    cur = Some(end);
                }
            }
            Seg::Z => d.push('Z'),
        }
    }
    d
}

/// The SVG of a display list.
pub fn to_svg(list: &DisplayList, options: &SvgOptions) -> String {
    let (w, h) = (list.size.width, list.size.height);
    let mut out = String::with_capacity(4096 + list.prims.len() * 160);
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}mm" height="{}mm" viewBox="0 0 {w} {h}">"#,
        thousandths_text(i64::from(w)),
        thousandths_text(i64::from(h))
    );
    if let Some(t) = &options.title {
        let _ = writeln!(out, "<title>{}</title>", esc(t));
    }
    let _ = writeln!(
        out,
        r#"<rect width="{w}" height="{h}"{}/>"#,
        paint("fill", &list.paper)
    );
    let mut clip = 0usize;
    for p in &list.prims {
        match p {
            Prim::Rect(r) => {
                let rr = &r.rect;
                let radius = if r.radius > 0 {
                    format!(r#" rx="{0}" ry="{0}""#, r.radius)
                } else {
                    String::new()
                };
                let _ = writeln!(
                    out,
                    r#"<rect x="{}" y="{}" width="{}" height="{}"{radius}{}{}/>"#,
                    rr.left,
                    rr.top,
                    rr.width,
                    rr.height,
                    fill_stroke(r.fill.as_deref(), r.stroke.as_ref()),
                    rotation(r.rotation, rr)
                );
            }
            Prim::Path(pp) => {
                let rule = if pp.even_odd {
                    r#" fill-rule="evenodd""#
                } else {
                    ""
                };
                let _ = writeln!(
                    out,
                    r#"<path d="{}"{}{rule}/>"#,
                    path_d(&pp.segments),
                    fill_stroke(pp.fill.as_deref(), pp.stroke.as_ref())
                );
            }
            Prim::Text(t) => {
                let italic = if t.italic {
                    r#" font-style="italic""#
                } else {
                    ""
                };
                let halo = match &t.halo {
                    Some(c) => format!(
                        r#"{} stroke-width="{}" stroke-linejoin="round" paint-order="stroke""#,
                        paint("stroke", c),
                        (t.size / 5).max(1)
                    ),
                    None => String::new(),
                };
                let _ = writeln!(
                    out,
                    r#"<text x="{}" y="{}" font-family="{}" font-size="{}" font-weight="{}"{italic}{}{halo}{} xml:space="preserve">{}</text>"#,
                    t.at[0],
                    t.at[1],
                    css_family(&t.font),
                    t.size,
                    t.weight,
                    paint("fill", &t.color),
                    point_rotation(t.rotation, t.at),
                    text_content(t)
                );
            }
            Prim::Image(i) => match options.assets.iter().find(|a| a.sha256 == i.asset) {
                Some(a) => {
                    let op = if i.opacity < 100 {
                        format!(r#" opacity="{}""#, unit_text(u32::from(i.opacity), 100))
                    } else {
                        String::new()
                    };
                    let _ = writeln!(
                        out,
                        r#"<image href="{}" x="{}" y="{}" width="{}" height="{}" preserveAspectRatio="none"{op}{}/>"#,
                        esc(&a.href),
                        i.rect.left,
                        i.rect.top,
                        i.rect.width,
                        i.rect.height,
                        rotation(i.rotation, &i.rect)
                    );
                }
                None => {
                    let _ = writeln!(
                        out,
                        r##"<rect x="{}" y="{}" width="{}" height="{}" fill="#f2f2f2" stroke="#b4b4b4" stroke-width="180"{}/>"##,
                        i.rect.left,
                        i.rect.top,
                        i.rect.width,
                        i.rect.height,
                        rotation(i.rotation, &i.rect)
                    );
                }
            },
            Prim::Map(m) => {
                let r = &m.clip;
                match options.maps.iter().find(|x| x.item == m.item) {
                    Some(img) => {
                        let _ = writeln!(
                            out,
                            r#"<image href="{}" x="{}" y="{}" width="{}" height="{}" preserveAspectRatio="none"{}/>"#,
                            esc(&img.href),
                            r.left,
                            r.top,
                            r.width,
                            r.height,
                            rotation(m.rotation, r)
                        );
                    }
                    None if options.placeholders => {
                        let rot = rotation(m.rotation, r);
                        let _ = writeln!(
                            out,
                            r##"<rect x="{}" y="{}" width="{}" height="{}" fill="#ececec"{rot}/>"##,
                            r.left, r.top, r.width, r.height
                        );
                        let size = (r.height.min(r.width) / 12).clamp(2_000, 12_000);
                        let cx = i64::from(r.left) + i64::from(r.width) / 2;
                        let cy = i64::from(r.top) + i64::from(r.height) / 2 + i64::from(size) / 3;
                        let _ = writeln!(
                            out,
                            r##"<text x="{cx}" y="{cy}" font-family="{}" font-size="{size}" fill="#a0a0a0" text-anchor="middle"{rot}>harita</text>"##,
                            css_family("barlow")
                        );
                    }
                    None => {}
                }
            }
            Prim::PushClip(c) => {
                clip += 1;
                let r = &c.rect;
                let _ = writeln!(
                    out,
                    r#"<clipPath id="k{clip}"><rect x="{}" y="{}" width="{}" height="{}"{}/></clipPath><g clip-path="url(#k{clip})">"#,
                    r.left,
                    r.top,
                    r.width,
                    r.height,
                    rotation(c.rotation, r)
                );
            }
            Prim::PopClip(_) | Prim::PopGroup(_) => out.push_str("</g>\n"),
            Prim::PushGroup(g) => {
                let _ = writeln!(
                    out,
                    r#"<g opacity="{}">"#,
                    unit_text(u32::from(g.opacity), 100)
                );
            }
        }
    }
    out.push_str("</svg>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_colours_are_written_one_way() {
        assert_eq!(unit_text(128, 255), "0.5");
        assert_eq!(unit_text(85, 100), "0.85");
        assert_eq!(unit_text(100, 100), "1");
        assert_eq!(
            paint("fill", "#ff000080"),
            r##" fill="#ff0000" fill-opacity="0.5""##
        );
        assert_eq!(esc("a<b & \"c\""), "a&lt;b &amp; &quot;c&quot;");
        let r = RectUm::new(0, 0, 3, 3);
        assert_eq!(rotation(90_000, &r), r#" transform="rotate(90 1.5 1.5)""#);
    }

    #[test]
    fn a_letter_the_face_lacks_is_named_with_the_face_that_draws_it() {
        let t = TextPrim {
            item: "t".into(),
            text: "Kot ↑ 12".into(),
            at: [0, 0],
            font: "barlow".into(),
            size: 2_500,
            weight: 400,
            italic: false,
            color: "#000000".into(),
            rotation: 0,
            width: 0,
            halo: None,
        };
        assert_eq!(
            text_content(&t),
            r#"Kot <tspan font-family="Arimo, Arial, sans-serif" font-weight="400">↑</tspan> 12"#
        );
    }

    #[test]
    fn a_full_circle_is_two_arcs() {
        let a = ArcSeg {
            center: [100, 100],
            radius: [50, 50],
            rotation: 0,
            start: 0,
            sweep: 360_000,
        };
        let d = path_d(&[Seg::M([150, 100]), Seg::A(a), Seg::Z]);
        assert_eq!(d, "M150 100A50 50 0 0 1 50 100A50 50 0 0 1 150 100Z");
    }
}
