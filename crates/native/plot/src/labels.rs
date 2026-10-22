//! Map text from the shared spatial/paragraph layout, without a window or UI font system (CAD-SHEET-06).
use kentos_contracts::{DrawingFont, Entity, TextFace};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::{
    Vec2,
    text::paragraph::{self, Run, Script},
};
use kentos_interaction::{Format, LabelSpot, Spatial};
use kentos_sheet::{display::MapPrim, pdf::TextAnchor};

use crate::maps::{MapLabel, PX_MM};

fn slot(s: &LabelSpot) -> Slot {
    match s {
        LabelSpot::Dimension { slot, .. }
        | LabelSpot::Text { slot, .. }
        | LabelSpot::Placed { slot, .. }
        | LabelSpot::PlacedLine { slot, .. }
        | LabelSpot::PlacedLetter { slot, .. }
        | LabelSpot::PlacedCallout { slot, .. }
        | LabelSpot::PieceText { slot, .. }
        | LabelSpot::PieceDimension { slot, .. }
        | LabelSpot::Line { slot, .. }
        | LabelSpot::ParagraphMask { slot, .. }
        | LabelSpot::Cell { slot, .. } => *slot,
    }
}

fn paper_color(c: Option<&str>) -> String {
    match c {
        Some("fg" | "label" | "ink") | None => "#111111".into(),
        Some("fg-dim") => "#68717d".into(),
        Some("paper") => "#ffffff".into(),
        Some(c) => c.to_owned(),
    }
}

fn label(layer: &str, font: DrawingFont) -> MapLabel {
    MapLabel {
        layer: layer.into(),
        text: String::new(),
        at: Vec2::new(0., 0.),
        anchor: TextAnchor::LeftBaseline,
        rotation: 0.,
        size: 0.,
        weight: 400,
        italic: false,
        font: Some(font),
        color: "#111111".into(),
        halo: "#ffffff".into(),
        mask: None,
        underline: None,
        fill: None,
        stroke: None,
        width_factor: 1.,
        lean: 0.,
    }
}

fn face(l: &mut MapLabel, f: &TextFace, legacy_italic: bool) {
    if let Some(font) = f.font {
        l.font = Some(font);
        l.weight = if f.bold { 600 } else { 400 };
        l.italic = f.italic;
        l.lean = f.oblique.unwrap_or(0.).to_radians().tan();
    } else {
        l.italic = legacy_italic;
    }
    if l.italic
        && !crate::fonts::FACE_KEYS
            .iter()
            .any(|(id, _, italic)| *italic && Some(*id) == l.font.map(crate::fonts::sheet_font))
    {
        l.lean += 0.25;
    }
}

fn ring(at: Vec2, rotation: f64, width: f64, height: f64, lean: f64) -> Vec<[f64; 2]> {
    let (s, c) = rotation.to_radians().sin_cos();
    [[0., 0.], [width, 0.], [width, height], [0., height]]
        .map(|[x, y]| {
            let x = x + y * lean;
            [at.x + c * x - s * y, at.y + s * x + c * y]
        })
        .to_vec()
}

fn width(l: &MapLabel, height: f64) -> f64 {
    let style = kentos_sheet::style::TextStyle {
        font: l.font.map_or("barlow", crate::fonts::sheet_font).into(),
        size: 1_000_000,
        weight: l.weight,
        italic: l.italic,
        color: l.color.clone(),
    };
    kentos_sheet::text::text_width(&l.text, &style) as f64 / 1_000_000. * height * l.width_factor
}

fn mask(l: &mut MapLabel, ground_height: f64, ground_width: f64, centered: bool) {
    let w = if ground_width > 0. {
        ground_width
    } else {
        width(l, ground_height)
    };
    let (s, c) = l.rotation.to_radians().sin_cos();
    let x = if centered { -w / 2. } else { 0. };
    let margin = ground_height * 0.1;
    let offset = Vec2::new(
        l.at.x + c * (x - margin) + s * ground_height * 0.23,
        l.at.y + s * (x - margin) - c * ground_height * 0.23,
    );
    l.mask = Some(ring(
        offset,
        l.rotation,
        w + margin * 2.,
        ground_height * 1.38,
        l.lean,
    ));
}

/// The map's text at its physical print size; layer selection/freeze/plot overrides are shared with vector paths.
pub fn map_labels(doc: &Document, spatial: &Spatial, m: &MapPrim) -> Vec<MapLabel> {
    labels_in_view(
        doc,
        spatial,
        &m.view,
        &m.layers,
        &m.layer_overrides,
        m.extent,
    )
}

/// Text collector shared by the desktop PDF and headless map export.
pub fn labels_in_view(
    doc: &Document,
    spatial: &Spatial,
    view: &kentos_sheet::display::MapViewPrim,
    layers: &kentos_sheet::kinds::MapLayers,
    overrides: &[kentos_sheet::kinds::LayerOverride],
    extent: Option<[f64; 4]>,
) -> Vec<MapLabel> {
    let Some(bounds) = extent else {
        return Vec::new();
    };
    let scale = view.scale.max(1e-6);
    let pixels = kentos_sheet::projection::Scale::for_rendering(scale)
        .pixels_per_metre(1. / (PX_MM * 1000.));
    let shown = crate::layers::map_layers(
        doc.layers().nodes(),
        &doc.settings().layer_states,
        layers,
        overrides,
    );
    let visible: std::collections::HashMap<_, _> =
        shown.iter().map(|n| (n.id.as_str(), n)).collect();
    let spots = spatial.labels(
        Vec2::new(bounds[0], bounds[1]),
        Vec2::new(bounds[2], bounds[3]),
        pixels,
        Default::default(),
    );
    let format = Format::of(doc.settings());
    let font = doc.settings().drawing_font.unwrap_or(DrawingFont::Barlow);
    let mut out = Vec::new();
    let mut placed: Option<(Slot, kentos_contracts::LabelStyle)> = None;
    let mut filled = std::collections::HashSet::new();
    for spot in spots {
        let id = slot(&spot);
        let Some(entity) = doc.get(id) else { continue };
        let Some(layer) = visible.get(entity.base().layer_id.as_str()) else {
            continue;
        };
        let mut l = label(&layer.id, font);
        let ground_height;
        let mut ground_mask = 0.;
        let mut centered = false;
        let mut shadow = None;
        match (&spot, entity) {
            (
                LabelSpot::Dimension {
                    at,
                    angle,
                    value,
                    unit,
                    prefix,
                    mask: masked,
                    ..
                },
                Entity::Dimension(d),
            ) => {
                l.at = *at;
                l.rotation = *angle;
                ground_height = d.height;
                centered = true;
                l.text = format.dimension_value(
                    prefix,
                    unit,
                    *value,
                    &d.look,
                    d.text.as_deref().filter(|t| !t.is_empty()),
                );
                l.font = Some(d.look.font.unwrap_or(font));
                l.weight = 500;
                l.anchor = TextAnchor::CenterBaseline;
                l.color = paper_color(
                    d.look
                        .text_color
                        .as_deref()
                        .or(entity.base().color.as_deref())
                        .or(Some(&layer.style.color)),
                );
                if *masked {
                    ground_mask = -1.;
                }
            }
            (
                LabelSpot::Text {
                    at,
                    rotation,
                    width_factor,
                    mask,
                    ..
                },
                Entity::Text(t),
            ) => {
                l.at = *at;
                l.rotation = *rotation;
                ground_height = t.height;
                ground_mask = *mask;
                l.width_factor = *width_factor;
                l.text = t.text.clone();
                face(&mut l, &t.face, true);
            }
            (
                LabelSpot::Text {
                    at, rotation, mask, ..
                },
                Entity::Leader(t),
            ) => {
                l.at = *at;
                l.rotation = *rotation;
                ground_height = t.height;
                ground_mask = *mask;
                l.text = t.text.clone().unwrap_or_default();
                face(&mut l, &TextFace::default(), true);
            }
            (
                LabelSpot::PieceText {
                    at,
                    rotation,
                    height,
                    text,
                    attribute,
                    width_factor,
                    mask,
                    face: f,
                    ..
                },
                _,
            ) => {
                l.at = *at;
                l.rotation = *rotation;
                ground_height = *height;
                ground_mask = *mask;
                l.width_factor = *width_factor;
                l.text = kentos_interaction::spatial::shown_text(
                    text,
                    attribute.as_deref(),
                    &entity.base().attrs,
                )
                .into();
                face(
                    &mut l,
                    &kentos_native_application::geometry::contract_face(f.clone()),
                    true,
                );
            }
            (
                LabelSpot::PieceDimension {
                    at,
                    angle,
                    height,
                    text,
                    value,
                    unit,
                    prefix,
                    mask: masked,
                    look,
                    ..
                },
                _,
            ) => {
                let look = kentos_native_application::geometry::contract_look(look.clone());
                l.at = *at;
                l.rotation = *angle;
                ground_height = *height;
                centered = true;
                l.anchor = TextAnchor::CenterBaseline;
                l.weight = 500;
                l.font = Some(look.font.unwrap_or(font));
                l.color = paper_color(
                    look.text_color
                        .as_deref()
                        .or(entity.base().color.as_deref())
                        .or(Some(&layer.style.color)),
                );
                l.text = format.dimension_value(
                    prefix,
                    unit,
                    *value,
                    &look,
                    text.as_deref().filter(|t| !t.is_empty()),
                );
                if *masked {
                    ground_mask = -1.;
                }
            }
            (
                LabelSpot::Line {
                    at,
                    rotation,
                    height,
                    width_factor,
                    start,
                    end,
                    piece,
                    face: f,
                    ..
                },
                _,
            ) => {
                let (text, runs, f) = match (piece, entity) {
                    (Some(p), _) => (
                        p.0.clone(),
                        p.1.clone(),
                        f.as_ref()
                            .map(|f| kentos_native_application::geometry::contract_face(f.clone()))
                            .unwrap_or_default(),
                    ),
                    (None, Entity::Text(t)) => (
                        t.text.clone(),
                        kentos_native_application::geometry::core_runs(&t.paragraph.runs)
                            .unwrap_or_default(),
                        t.face.clone(),
                    ),
                    _ => continue,
                };
                l.at = *at;
                l.rotation = *rotation;
                l.width_factor = *width_factor;
                face(&mut l, &f, false);
                out.extend(line(l, &text, &runs, *start, *end, *height, scale));
                continue;
            }
            (
                LabelSpot::ParagraphMask {
                    at,
                    rotation,
                    width,
                    height,
                    lean,
                    ..
                },
                _,
            ) => {
                l.fill = Some(ring(*at, *rotation, *width, *height, *lean));
                l.color = "#ffffff".into();
                out.push(l);
                continue;
            }
            (
                LabelSpot::Cell {
                    at,
                    rotation,
                    row,
                    col,
                    bold,
                    ..
                },
                Entity::Table(t),
            ) => {
                if filled.insert(id)
                    && let Some(g) = kentos_geometry_core::geom::table::table_geom(
                        &kentos_native_application::geometry::shape(entity),
                    )
                {
                    for fill in g.fills() {
                        let mut p = l.clone();
                        p.fill = Some(fill.corners.map(|p| [p.x, p.y]).to_vec());
                        p.color = fill.fill;
                        out.push(p);
                    }
                }
                let Some(text) = t.cells.get(*row).and_then(|r| r.get(*col)) else {
                    continue;
                };
                l.text = text.clone();
                l.at = *at;
                l.rotation = *rotation;
                ground_height = t.height;
                face(&mut l, &t.face, false);
                if *bold {
                    l.weight = 600;
                }
                l.color = paper_color(
                    t.cell_style(*row, *col)
                        .color
                        .as_deref()
                        .or(entity.base().color.as_deref())
                        .or(Some(&layer.style.color)),
                );
            }
            (
                LabelSpot::Placed {
                    at,
                    angle,
                    width,
                    height,
                    class,
                    state,
                    ..
                },
                _,
            ) => {
                use kentos_geometry_core::labels::engine::{HIDDEN, UNPLACED};
                if state & (HIDDEN | UNPLACED) != 0 {
                    placed = None;
                    continue;
                }
                let style = match &layer.style.labels {
                    Some(r) if r.mode == kentos_contracts::LabelsMode::Rules => {
                        r.classes.get(*class as usize).map(|c| c.style.clone())
                    }
                    _ => layer
                        .style
                        .label
                        .clone()
                        .or_else(|| kentos_interaction::spatial::default_label(entity.kind())),
                };
                if let Some(s) = &style {
                    backdrop(
                        &mut out,
                        &l,
                        s,
                        *at,
                        *angle,
                        *width / pixels,
                        *height / pixels,
                        scale,
                    );
                }
                placed = style.map(|s| (id, s));
                continue;
            }
            (
                LabelSpot::PlacedLine {
                    at,
                    angle,
                    size,
                    text,
                    ..
                },
                _,
            ) => {
                let Some((_, style)) = placed.as_ref().filter(|(slot, _)| *slot == id) else {
                    continue;
                };
                l.at = *at;
                l.rotation = *angle;
                l.text = text.clone();
                ground_height = *size / pixels;
                l.anchor = TextAnchor::CenterMiddle;
                placed_look(&mut l, style);
                shadow = style.shadow.clone();
            }
            (
                LabelSpot::PlacedLetter {
                    at,
                    angle,
                    size,
                    letter,
                    advance,
                    ..
                },
                _,
            ) => {
                let Some((_, style)) = placed.as_ref().filter(|(slot, _)| *slot == id) else {
                    continue;
                };
                backdrop(
                    &mut out,
                    &l,
                    style,
                    *at,
                    *angle,
                    *advance / pixels,
                    *size / pixels,
                    scale,
                );
                l.at = *at;
                l.rotation = *angle;
                l.text = letter.to_string();
                ground_height = *size / pixels;
                l.anchor = TextAnchor::CenterMiddle;
                placed_look(&mut l, style);
                shadow = style.shadow.clone();
            }
            (LabelSpot::PlacedCallout { from, to, .. }, _) => {
                let Some((_, style)) = placed.as_ref().filter(|(slot, _)| *slot == id) else {
                    continue;
                };
                let manhattan = style
                    .callout
                    .as_ref()
                    .is_some_and(|c| c.kind == kentos_contracts::CalloutKind::Manhattan);
                let points = if manhattan {
                    vec![[from.x, from.y], [to.x, from.y], [to.x, to.y]]
                } else {
                    vec![[from.x, from.y], [to.x, to.y]]
                };
                l.color = paper_color(
                    style
                        .callout
                        .as_ref()
                        .and_then(|c| c.color.as_deref())
                        .or(style.color.as_deref()),
                );
                l.stroke = Some((points, false));
                out.push(l);
                continue;
            }
            _ => continue,
        }
        l.size = kentos_sheet::projection::Scale::for_rendering(scale).paper_mm(ground_height);
        if ground_mask != 0. {
            mask(&mut l, ground_height, ground_mask, centered);
        }
        if !l.text.is_empty() && l.size > 0. {
            if let Some(shadow) = shadow {
                let mut sh = l.clone();
                let k = kentos_sheet::projection::Scale::for_rendering(scale)
                    .model_metres_from_mm(PX_MM);
                sh.at.x += shadow.dx * k;
                sh.at.y += shadow.dy * k;
                sh.color = alpha(
                    &paper_color(shadow.color.as_deref().or(Some("#000000"))),
                    shadow.opacity.unwrap_or(0.5),
                );
                sh.halo = "#00000000".into();
                out.push(sh);
            }
            out.push(l);
        }
    }
    out
}

fn alpha(color: &str, opacity: f64) -> String {
    let p = kentos_native_style::color::parse_hex(color, opacity);
    format!(
        "#{:02x}{:02x}{:02x}{:02x}",
        (p[0] * 255.).round() as u8,
        (p[1] * 255.).round() as u8,
        (p[2] * 255.).round() as u8,
        (p[3] * 255.).round() as u8
    )
}

fn placed_look(l: &mut MapLabel, s: &kentos_contracts::LabelStyle) {
    l.weight = s.weight.unwrap_or(500);
    l.italic = s.italic.unwrap_or(false);
    l.color = paper_color(s.color.as_deref().or(
        if s.ink == Some(kentos_contracts::LabelInk::FgDim) {
            Some("fg-dim")
        } else {
            None
        },
    ));
    l.halo = if s.background.as_ref().is_some_and(|b| b.fill.is_some())
        || s.halo.as_ref().is_some_and(|h| h.width <= 0.)
    {
        "#00000000".into()
    } else {
        paper_color(
            s.halo
                .as_ref()
                .and_then(|h| h.color.as_deref())
                .or(Some("paper")),
        )
    };
}

#[allow(clippy::too_many_arguments)]
fn backdrop(
    out: &mut Vec<MapLabel>,
    base: &MapLabel,
    s: &kentos_contracts::LabelStyle,
    at: Vec2,
    rotation: f64,
    w: f64,
    h: f64,
    scale: f64,
) {
    let Some(back) = &s.background else { return };
    let px = kentos_sheet::projection::Scale::for_rendering(scale).model_metres_from_mm(PX_MM);
    let pad = back.padding.unwrap_or(2.) * px;
    let (sn, cs) = rotation.to_radians().sin_cos();
    let (hw, hh) = (w / 2. + pad, h / 2. + pad);
    let p = |x: f64, y: f64| [at.x + cs * x - sn * y, at.y + sn * x + cs * y];
    let points = match back.shape {
        kentos_contracts::LabelShape::Rect => vec![p(-hw, -hh), p(hw, -hh), p(hw, hh), p(-hw, hh)],
        kentos_contracts::LabelShape::Ellipse => (0..32)
            .map(|i| {
                let t = i as f64 / 32. * std::f64::consts::TAU;
                p(hw * 2f64.sqrt() * t.cos(), hh * 2f64.sqrt() * t.sin())
            })
            .collect(),
        kentos_contracts::LabelShape::Round => {
            let r = hh.min(6. * px).min(hw);
            let mut pts = Vec::new();
            for (cx, cy, from) in [
                (hw - r, hh - r, 0f64),
                (-hw + r, hh - r, 90.),
                (-hw + r, -hh + r, 180.),
                (hw - r, -hh + r, 270.),
            ] {
                for i in 0..=5 {
                    let t = (from + i as f64 * 18.).to_radians();
                    pts.push(p(cx + r * t.cos(), cy + r * t.sin()));
                }
            }
            pts
        }
    };
    if back.fill.is_some()
        && let Some(shadow) = &s.shadow
    {
        let mut l = base.clone();
        l.color = alpha(
            &paper_color(shadow.color.as_deref().or(Some("#000000"))),
            shadow.opacity.unwrap_or(0.5),
        );
        l.fill = Some(
            points
                .iter()
                .map(|p| [p[0] + shadow.dx * px, p[1] + shadow.dy * px])
                .collect(),
        );
        out.push(l);
    }
    if let Some(color) = &back.fill {
        let mut l = base.clone();
        l.color = paper_color(Some(color));
        l.fill = Some(points.clone());
        out.push(l);
    }
    if let Some(color) = &back.stroke {
        let mut l = base.clone();
        l.color = paper_color(Some(color));
        l.stroke = Some((points, true));
        out.push(l);
    }
}

fn line(
    base: MapLabel,
    text: &str,
    runs: &[Run],
    start: usize,
    end: usize,
    height: f64,
    scale: f64,
) -> Vec<MapLabel> {
    let chars: Vec<char> = text.chars().collect();
    let end = end.min(chars.len());
    let mut i = start.min(end);
    let mut x = 0.;
    let mut out = Vec::new();
    let font = kentos_native_application::geometry::drawing_font(base.font);
    let (s, c) = base.rotation.to_radians().sin_cos();
    let em = height * base.width_factor / 1000.;
    while i < end {
        let run = runs
            .iter()
            .find(|r| r.start as usize <= i && i < (r.end as usize));
        let mut j = i + 1;
        while j < end
            && runs
                .iter()
                .find(|r| r.start as usize <= j && j < (r.end as usize))
                == run
        {
            j += 1;
        }
        let bold = base.weight >= 600 || run.is_some_and(|r| r.bold);
        let script = run.and_then(|r| r.script);
        let mut pieces: Vec<(String, f64, f64, f64, f64)> = Vec::new();
        let stack = run
            .filter(|r| {
                script == Some(Script::Stack) && r.start as usize >= i && r.end as usize <= j
            })
            .and_then(|r| paragraph::stack_parts(&chars, r, font, bold));
        let advance;
        if let Some((q, mark, num, den)) = stack {
            let upper: String = chars[i..q].iter().collect();
            let lower: String = chars[q + 1..j].iter().collect();
            if mark == '#' {
                let slash = paragraph::advance(font, '/', bold, true);
                pieces.push((
                    upper,
                    x,
                    paragraph::STACK_OVER * height,
                    paragraph::SCRIPT_SIZE,
                    0.,
                ));
                pieces.push(("/".into(), x + num * em, 0., paragraph::SCRIPT_SIZE, 0.));
                pieces.push((
                    lower,
                    x + (num + slash) * em,
                    paragraph::STACK_UNDER * height,
                    paragraph::SCRIPT_SIZE,
                    0.,
                ));
                advance = (num + slash + den) * em;
            } else {
                let w = num.max(den);
                pieces.push((
                    upper,
                    x + (w - num) / 2. * em,
                    paragraph::STACK_OVER * height,
                    paragraph::SCRIPT_SIZE,
                    0.,
                ));
                pieces.push((
                    lower,
                    x + (w - den) / 2. * em,
                    paragraph::STACK_UNDER * height,
                    paragraph::SCRIPT_SIZE,
                    0.,
                ));
                if mark == '/' {
                    pieces.push((
                        String::new(),
                        x,
                        paragraph::STACK_BAR * height + 0.12 * height * paragraph::SCRIPT_SIZE,
                        paragraph::SCRIPT_SIZE,
                        w * em,
                    ));
                }
                advance = w * em;
            }
        } else {
            advance = chars[i..j]
                .iter()
                .map(|ch| paragraph::advance(font, *ch, bold, script.is_some()))
                .sum::<f64>()
                * em;
            let lift = match script {
                Some(Script::Super) => 0.4 * height,
                Some(Script::Sub) => -0.15 * height,
                _ => 0.,
            };
            pieces.push((
                chars[i..j].iter().collect(),
                x,
                lift,
                if script.is_some() { 0.6 } else { 1. },
                if run.is_some_and(|r| r.underline) {
                    advance
                } else {
                    0.
                },
            ));
        }
        for (text, along, lift, k, underline) in pieces {
            let mut l = base.clone();
            l.weight = if bold { 600 } else { 400 };
            l.italic |= run.is_some_and(|r| r.italic);
            if let Some(color) = run.and_then(|r| r.color.as_deref()) {
                l.color = paper_color(Some(color));
            }
            let along = along + lift * base.lean;
            l.at = Vec2::new(
                base.at.x + c * along - s * lift,
                base.at.y + s * along + c * lift,
            );
            l.text = text;
            l.size = kentos_sheet::projection::Scale::for_rendering(scale).paper_mm(height * k);
            if underline > 0. {
                let d = height * k * 0.12;
                let start = Vec2::new(l.at.x + s * d, l.at.y - c * d);
                l.underline = Some(ring(start, l.rotation, underline, height * k * 0.06, 0.));
            }
            out.push(l);
        }
        x += advance;
        i = j;
    }
    out
}
