//! An SVG drawing read into the atlas's vector picture (docs/adr/0090):
//! what the browser does when the web's atlas draws a library SVG with
//! Canvas2D. The file is parsed with roxmltree; colours, transforms, lengths,
//! the viewBox and path data are the SVG core's readers (`kentos-svg-core`),
//! the same the editor's importer uses.
//!
//! Read: `<svg>` with its viewBox, `<g>`, `<use>` (href and xlink:href),
//! `<defs>`, `<symbol>`, luminance `<mask>`s, every basic shape, paths and
//! texts (glyph outlines from the host's text system); presentation
//! attributes and `style` with inheritance, `opacity`, `fill-opacity`,
//! `stroke-opacity`, dashes, caps, joins and the fill rule. Style sheets,
//! gradients, patterns, clip paths, filters and embedded images are left out
//! (the web's editor imports them the same way: sheets aside, they become flat
//! colours or are skipped); the pictograms KentOS ships use none of them.

use std::collections::HashMap;

use kentos_render_wgpu::styled::picture::{
    Fill, LineCap, LineJoin, Matrix, Node, Picture, Segment, Stroke,
};
use kentos_svg_core::path::{self, IDENTITY, multiply, parse_path_data};
use kentos_svg_core::shape::SubPath;
use kentos_svg_core::values::{parse_decls, read_color, read_transform, to_user, view_box_of};

/// Glyph outlines for SVG texts: the text, its font stack, weight and italic,
/// giving the outline at its reference size (see `TextOutline`).
pub type Glyphs<'a> = &'a dyn Fn(
    &str,
    Option<&str>,
    f64,
    bool,
)
    -> Option<std::sync::Arc<kentos_render_wgpu::styled::TextOutline>>;

/// Inherited properties.
#[derive(Clone, Debug)]
struct Props {
    fill: Option<String>,
    stroke: Option<String>,
    color: String,
    stroke_width: f64,
    fill_opacity: f64,
    stroke_opacity: f64,
    fill_rule_even_odd: bool,
    cap: LineCap,
    join: LineJoin,
    miter: f64,
    dash: Option<Vec<f64>>,
    dash_offset: f64,
    font_size: f64,
    font_family: Option<String>,
    font_weight: f64,
    italic: bool,
    anchor: f64,
    visible: bool,
}

impl Default for Props {
    fn default() -> Self {
        Props {
            fill: Some("#000000".into()),
            stroke: None,
            color: "#000000".into(),
            stroke_width: 1.0,
            fill_opacity: 1.0,
            stroke_opacity: 1.0,
            fill_rule_even_odd: false,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter: 4.0,
            dash: None,
            dash_offset: 0.0,
            font_size: 16.0,
            font_family: None,
            font_weight: 400.0,
            italic: false,
            anchor: 0.0,
            visible: true,
        }
    }
}

/// An element's own declarations: presentation attributes, then `style` over them.
fn declarations(node: roxmltree::Node<'_, '_>) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = node
        .attributes()
        .map(|a| (a.name().to_owned(), a.value().to_owned()))
        .collect();
    if let Some(style) = node.attribute("style") {
        for d in parse_decls(style) {
            out.push((d.prop.clone(), d.value.clone()));
        }
    }
    out
}

fn number(v: &str) -> Option<f64> {
    to_user(Some(v), 0.0, 16.0).filter(|x| x.is_finite())
}

/// Applies the inherited properties an element sets.
fn inherit(p: &Props, decls: &[(String, String)]) -> Props {
    let mut p = p.clone();
    for (k, v) in decls {
        let v = v.trim();
        match k.as_str() {
            "fill" => p.fill = (v != "none").then(|| v.to_owned()),
            "stroke" => p.stroke = (v != "none").then(|| v.to_owned()),
            "color" => p.color = v.to_owned(),
            "stroke-width" => p.stroke_width = number(v).unwrap_or(p.stroke_width),
            "fill-opacity" => p.fill_opacity = v.parse().unwrap_or(p.fill_opacity),
            "stroke-opacity" => p.stroke_opacity = v.parse().unwrap_or(p.stroke_opacity),
            "fill-rule" => p.fill_rule_even_odd = v == "evenodd",
            "stroke-linecap" => {
                p.cap = match v {
                    "round" => LineCap::Round,
                    "square" => LineCap::Square,
                    _ => LineCap::Butt,
                }
            }
            "stroke-linejoin" => {
                p.join = match v {
                    "round" => LineJoin::Round,
                    "bevel" => LineJoin::Bevel,
                    _ => LineJoin::Miter,
                }
            }
            "stroke-miterlimit" => p.miter = v.parse().unwrap_or(p.miter),
            "stroke-dasharray" => {
                let d: Vec<f64> = v
                    .split(|c: char| c == ',' || c.is_whitespace())
                    .filter(|s| !s.is_empty())
                    .filter_map(number)
                    .collect();
                p.dash = (!d.is_empty() && d.iter().any(|x| *x > 0.0)).then_some(d);
            }
            "stroke-dashoffset" => p.dash_offset = number(v).unwrap_or(0.0),
            "font-size" => p.font_size = number(v).unwrap_or(p.font_size),
            "font-family" => p.font_family = Some(v.to_owned()),
            "font-weight" => {
                p.font_weight = match v {
                    "bold" | "bolder" => 700.0,
                    "normal" | "lighter" => 400.0,
                    other => other.parse().unwrap_or(p.font_weight),
                }
            }
            "font-style" => p.italic = v == "italic" || v == "oblique",
            "text-anchor" => {
                p.anchor = match v {
                    "middle" => 0.5,
                    "end" => 1.0,
                    _ => 0.0,
                }
            }
            "visibility" => p.visible = v != "hidden" && v != "collapse",
            _ => {}
        }
    }
    p
}

/// A paint as a colour with the opacity it carries; None for none or what cannot be read.
fn paint(value: Option<&str>, color: &str, opacity: f64) -> Option<[u8; 4]> {
    let value = value?;
    let value = if value.eq_ignore_ascii_case("currentcolor") {
        color
    } else {
        value
    };
    let c = read_color(value)?;
    let hex = c.hex.trim_start_matches('#');
    let channel = |i: usize| u8::from_str_radix(hex.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0);
    let a = (c.alpha * opacity).clamp(0.0, 1.0);
    Some([
        channel(0),
        channel(2),
        channel(4),
        (a * 255.0).round() as u8,
    ])
}

fn to_f32(m: &path::Matrix) -> Matrix {
    m.map(|x| x as f32)
}

/// A sub-path's nodes as segments: cubic where a node has a handle.
fn segments(subs: &[SubPath]) -> Vec<Segment> {
    let mut out = Vec::new();
    for sp in subs {
        let n = sp.nodes.len();
        let Some(first) = sp.nodes.first() else {
            continue;
        };
        out.push(Segment::Move([first.x as f32, first.y as f32]));
        let count = if sp.closed { n } else { n.saturating_sub(1) };
        for i in 0..count {
            let a = &sp.nodes[i];
            let b = &sp.nodes[(i + 1) % n];
            if a.out.is_some() || b.in_.is_some() {
                let c1 = a.out.unwrap_or([a.x, a.y]);
                let c2 = b.in_.unwrap_or([b.x, b.y]);
                out.push(Segment::Cubic(
                    [c1[0] as f32, c1[1] as f32],
                    [c2[0] as f32, c2[1] as f32],
                    [b.x as f32, b.y as f32],
                ));
            } else if !(sp.closed && i + 1 == n) {
                out.push(Segment::Line([b.x as f32, b.y as f32]));
            }
        }
        if sp.closed {
            out.push(Segment::Close);
        }
    }
    out
}

fn attr(node: roxmltree::Node<'_, '_>, k: &str) -> f64 {
    node.attribute(k).and_then(number).unwrap_or(0.0)
}

/// An ellipse as four cubics (y down).
fn ellipse(cx: f64, cy: f64, rx: f64, ry: f64) -> Vec<Segment> {
    const K: f64 = 0.552_284_749_830_793_4;
    let p = |x: f64, y: f64| [x as f32, y as f32];
    vec![
        Segment::Move(p(cx + rx, cy)),
        Segment::Cubic(
            p(cx + rx, cy + K * ry),
            p(cx + K * rx, cy + ry),
            p(cx, cy + ry),
        ),
        Segment::Cubic(
            p(cx - K * rx, cy + ry),
            p(cx - rx, cy + K * ry),
            p(cx - rx, cy),
        ),
        Segment::Cubic(
            p(cx - rx, cy - K * ry),
            p(cx - K * rx, cy - ry),
            p(cx, cy - ry),
        ),
        Segment::Cubic(
            p(cx + K * rx, cy - ry),
            p(cx + rx, cy - K * ry),
            p(cx + rx, cy),
        ),
        Segment::Close,
    ]
}

/// A rectangle, with rounded corners when it has them.
fn rect(x: f64, y: f64, w: f64, h: f64, rx: f64, ry: f64) -> Vec<Segment> {
    let p = |x: f64, y: f64| [x as f32, y as f32];
    if rx <= 0.0 || ry <= 0.0 {
        return vec![
            Segment::Move(p(x, y)),
            Segment::Line(p(x + w, y)),
            Segment::Line(p(x + w, y + h)),
            Segment::Line(p(x, y + h)),
            Segment::Close,
        ];
    }
    const K: f64 = 0.552_284_749_830_793_4;
    let (rx, ry) = (rx.min(w / 2.0), ry.min(h / 2.0));
    vec![
        Segment::Move(p(x + rx, y)),
        Segment::Line(p(x + w - rx, y)),
        Segment::Cubic(
            p(x + w - rx + K * rx, y),
            p(x + w, y + ry - K * ry),
            p(x + w, y + ry),
        ),
        Segment::Line(p(x + w, y + h - ry)),
        Segment::Cubic(
            p(x + w, y + h - ry + K * ry),
            p(x + w - rx + K * rx, y + h),
            p(x + w - rx, y + h),
        ),
        Segment::Line(p(x + rx, y + h)),
        Segment::Cubic(
            p(x + rx - K * rx, y + h),
            p(x, y + h - ry + K * ry),
            p(x, y + h - ry),
        ),
        Segment::Line(p(x, y + ry)),
        Segment::Cubic(p(x, y + ry - K * ry), p(x + rx - K * rx, y), p(x + rx, y)),
        Segment::Close,
    ]
}

fn points(node: roxmltree::Node<'_, '_>, closed: bool) -> Vec<Segment> {
    let v: Vec<f64> = node
        .attribute("points")
        .unwrap_or("")
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    let mut out = Vec::new();
    for (i, xy) in v.chunks_exact(2).enumerate() {
        let p = [xy[0] as f32, xy[1] as f32];
        out.push(if i == 0 {
            Segment::Move(p)
        } else {
            Segment::Line(p)
        });
    }
    if closed && !out.is_empty() {
        out.push(Segment::Close);
    }
    out
}

struct Reader<'a, 'input> {
    ids: HashMap<&'a str, roxmltree::Node<'a, 'input>>,
    glyphs: Glyphs<'a>,
    /// `<use>` depth, so a file referring to itself ends.
    depth: usize,
}

impl<'a, 'input> Reader<'a, 'input> {
    fn shape(
        &self,
        node: roxmltree::Node<'a, 'input>,
        p: &Props,
        m: &path::Matrix,
    ) -> Option<Node> {
        let segments = match node.tag_name().name() {
            "path" => segments(&parse_path_data(node.attribute("d").unwrap_or(""))),
            "rect" => {
                let (w, h) = (attr(node, "width"), attr(node, "height"));
                if w <= 0.0 || h <= 0.0 {
                    return None;
                }
                let rx = node.attribute("rx").and_then(number);
                let ry = node.attribute("ry").and_then(number);
                let (rx, ry) = (rx.or(ry).unwrap_or(0.0), ry.or(rx).unwrap_or(0.0));
                rect(attr(node, "x"), attr(node, "y"), w, h, rx, ry)
            }
            "circle" => {
                let r = attr(node, "r");
                if r <= 0.0 {
                    return None;
                }
                ellipse(attr(node, "cx"), attr(node, "cy"), r, r)
            }
            "ellipse" => {
                let (rx, ry) = (attr(node, "rx"), attr(node, "ry"));
                if rx <= 0.0 || ry <= 0.0 {
                    return None;
                }
                ellipse(attr(node, "cx"), attr(node, "cy"), rx, ry)
            }
            "line" => {
                let pt = |a: &str, b: &str| [attr(node, a) as f32, attr(node, b) as f32];
                vec![Segment::Move(pt("x1", "y1")), Segment::Line(pt("x2", "y2"))]
            }
            "polyline" => points(node, false),
            "polygon" => points(node, true),
            _ => return None,
        };
        Some(Node::Path {
            segments,
            transform: to_f32(m),
            fill: paint(p.fill.as_deref(), &p.color, p.fill_opacity).map(|color| Fill {
                color,
                even_odd: p.fill_rule_even_odd,
            }),
            stroke: paint(p.stroke.as_deref(), &p.color, p.stroke_opacity).map(|color| Stroke {
                color,
                width: p.stroke_width as f32,
                cap: p.cap,
                join: p.join,
                miter: p.miter as f32,
                dash: p
                    .dash
                    .as_ref()
                    .map(|d| (d.iter().map(|x| *x as f32).collect(), p.dash_offset as f32)),
            }),
        })
    }

    /// A text as glyph outlines at its place and anchor.
    fn text(&self, node: roxmltree::Node<'a, 'input>, p: &Props, m: &path::Matrix) -> Option<Node> {
        let content: String = node
            .descendants()
            .filter(|n| n.is_text())
            .filter_map(|n| n.text())
            .collect::<String>();
        let content = content.split_whitespace().collect::<Vec<_>>().join(" ");
        if content.is_empty() {
            return None;
        }
        let outline = (self.glyphs)(&content, p.font_family.as_deref(), p.font_weight, p.italic)?;
        let k = p.font_size / f64::from(outline.size.max(1e-6));
        let x = attr(node, "x") - p.anchor * f64::from(outline.advance) * k;
        let y = attr(node, "y");
        let place = multiply(m, &[k, 0.0, 0.0, k, x, y]);
        let fill = paint(p.fill.as_deref(), &p.color, p.fill_opacity)?;
        let children = outline
            .glyphs
            .iter()
            .map(|g| Node::Path {
                segments: g.clone(),
                transform: to_f32(&place),
                fill: Some(Fill {
                    color: fill,
                    even_odd: false,
                }),
                stroke: None,
            })
            .collect();
        Some(Node::Group {
            opacity: 1.0,
            mask: None,
            children,
        })
    }

    /// The nodes of an element and what it holds.
    fn element(
        &mut self,
        node: roxmltree::Node<'a, 'input>,
        parent: &Props,
        m: &path::Matrix,
        out: &mut Vec<Node>,
    ) {
        let name = node.tag_name().name();
        if matches!(
            name,
            "defs"
                | "symbol"
                | "mask"
                | "clipPath"
                | "linearGradient"
                | "radialGradient"
                | "pattern"
                | "filter"
                | "style"
                | "title"
                | "desc"
                | "metadata"
                | "script"
                | "marker"
                | "image"
                | "foreignObject"
        ) {
            return;
        }
        let decls = declarations(node);
        if decls
            .iter()
            .any(|(k, v)| k == "display" && v.trim() == "none")
        {
            return;
        }
        let p = inherit(parent, &decls);
        let own = read_transform(node.attribute("transform"));
        let mut m = multiply(m, &own);
        let mut nodes = Vec::new();
        match name {
            "g" | "a" | "switch" => {
                for child in node.children().filter(|c| c.is_element()) {
                    self.element(child, &p, &m, &mut nodes);
                }
            }
            "svg" => {
                // A nested drawing: its place and viewBox.
                m = multiply(&m, &[1.0, 0.0, 0.0, 1.0, attr(node, "x"), attr(node, "y")]);
                if let Some(vb) = view_box_of(node.attribute("viewBox")) {
                    let w = node.attribute("width").and_then(number).unwrap_or(vb[2]);
                    let h = node.attribute("height").and_then(number).unwrap_or(vb[3]);
                    m = multiply(
                        &m,
                        &kentos_svg_core::values::view_box_transform(
                            &vb,
                            w,
                            h,
                            node.attribute("preserveAspectRatio").unwrap_or(""),
                        ),
                    );
                }
                for child in node.children().filter(|c| c.is_element()) {
                    self.element(child, &p, &m, &mut nodes);
                }
            }
            "use" => {
                let href = node
                    .attribute("href")
                    .or_else(|| node.attribute(("http://www.w3.org/1999/xlink", "href")));
                let target = href
                    .and_then(|h| h.strip_prefix('#'))
                    .and_then(|id| self.ids.get(id).copied());
                if let Some(target) = target
                    && self.depth < 16
                {
                    self.depth += 1;
                    let m = multiply(&m, &[1.0, 0.0, 0.0, 1.0, attr(node, "x"), attr(node, "y")]);
                    if target.tag_name().name() == "symbol" {
                        for child in target.children().filter(|c| c.is_element()) {
                            self.element(child, &p, &m, &mut nodes);
                        }
                    } else {
                        self.element(target, &p, &m, &mut nodes);
                    }
                    self.depth -= 1;
                }
            }
            "text" => {
                if p.visible
                    && let Some(n) = self.text(node, &p, &m)
                {
                    nodes.push(n);
                }
            }
            _ => {
                if p.visible
                    && let Some(n) = self.shape(node, &p, &m)
                {
                    nodes.push(n);
                }
            }
        }
        if nodes.is_empty() {
            return;
        }
        let opacity = decls
            .iter()
            .rev()
            .find(|(k, _)| k == "opacity")
            .and_then(|(_, v)| v.trim().parse::<f64>().ok())
            .unwrap_or(1.0)
            .clamp(0.0, 1.0);
        let mask = decls
            .iter()
            .rev()
            .find(|(k, _)| k == "mask")
            .and_then(|(_, v)| {
                let id = v.trim().strip_prefix("url(")?.trim_end_matches(')').trim();
                let id = id
                    .trim_matches(|c| c == '"' || c == '\'')
                    .strip_prefix('#')?;
                self.ids.get(id).copied()
            });
        if opacity >= 1.0 && mask.is_none() {
            out.extend(nodes);
            return;
        }
        let mask = mask.map(|mask| {
            // Mask content in the user space of the element it cuts (maskContentUnits = userSpaceOnUse).
            let mut content = Vec::new();
            let base = Props::default();
            for child in mask.children().filter(|c| c.is_element()) {
                self.element(child, &base, &m, &mut content);
            }
            content
        });
        out.push(Node::Group {
            opacity: opacity as f32,
            mask,
            children: nodes,
        });
    }
}

/// The picture of an SVG text; None when it is not a readable SVG drawing.
pub fn picture(svg: &str, glyphs: Glyphs<'_>) -> Option<Picture> {
    let doc = roxmltree::Document::parse_with_options(
        svg,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..roxmltree::ParsingOptions::default()
        },
    )
    .ok()?;
    let root = doc.root_element();
    if root.tag_name().name() != "svg" {
        return None;
    }
    let ids: HashMap<&str, roxmltree::Node<'_, '_>> = doc
        .descendants()
        .filter_map(|n| n.attribute("id").map(|id| (id, n)))
        .collect();
    let view = match view_box_of(root.attribute("viewBox")) {
        Some(vb) => [vb[0] as f32, vb[1] as f32, vb[2] as f32, vb[3] as f32],
        None => {
            let w = root.attribute("width").and_then(number).unwrap_or(100.0);
            let h = root.attribute("height").and_then(number).unwrap_or(100.0);
            [0.0, 0.0, w as f32, h as f32]
        }
    };
    let mut reader = Reader {
        ids,
        glyphs,
        depth: 0,
    };
    let base = inherit(&Props::default(), &declarations(root));
    let mut nodes = Vec::new();
    for child in root.children().filter(|c| c.is_element()) {
        reader.element(child, &base, &IDENTITY, &mut nodes);
    }
    Some(Picture::Vector { view, nodes })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_glyphs(
        _: &str,
        _: Option<&str>,
        _: f64,
        _: bool,
    ) -> Option<std::sync::Arc<kentos_render_wgpu::styled::TextOutline>> {
        None
    }

    #[test]
    fn reads_shapes_uses_and_masks() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><mask id="m"><rect width="100" height="100" fill="#fff"/></mask><g id="a" fill="currentColor"><path d="M0 0H10V10Z"/></g><use href="#a" x="20"/><circle cx="50" cy="50" r="5" mask="url(#m)" fill="#ff0000"/></svg>"##;
        let Some(Picture::Vector { view, nodes }) = picture(svg, &no_glyphs) else {
            panic!("an SVG picture");
        };
        assert_eq!(view, [0.0, 0.0, 100.0, 100.0]);
        assert_eq!(nodes.len(), 3, "{nodes:#?}");
        match &nodes[1] {
            Node::Path {
                transform, fill, ..
            } => {
                assert_eq!(transform[4], 20.0);
                assert_eq!(fill.map(|f| f.color), Some([0, 0, 0, 255]));
            }
            other => panic!("the use: {other:?}"),
        }
        assert!(matches!(&nodes[2], Node::Group { mask: Some(m), .. } if m.len() == 1));
    }

    #[test]
    fn every_system_drawing_reads() {
        let lib = kentos_native_style::system::library();
        let mut read = 0;
        for (item, _) in lib.items(None) {
            if item.format() == Some("svg") {
                let svg = item.data().unwrap_or("");
                assert!(picture(svg, &no_glyphs).is_some(), "{}", item.id());
                read += 1;
            }
        }
        assert!(read >= 81);
    }
}
