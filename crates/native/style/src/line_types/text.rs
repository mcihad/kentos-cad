//! Frozen contours for CAD linetype text. Shaped once with bundled fonts,
//! independent of the operating system and of the eventual render backend.
use kentos_contracts::{LineTypeDef, PatternUnit, ProjectSettings};
use kentos_geometry_core::geom::{
    arrangement::{Rule, Source},
    intersect::Edge,
    overlay::overlay,
};
use kentos_geometry_core::jsmath::{js_hypot, tan};
use kentos_geometry_core::{
    Vec2,
    entity::TextPlace,
    text::{
        Font, fonts,
        shaping::{self, OutlineOp, placed},
    },
};
use serde_json::{Value, json};

const TOL: f64 = 0.0001;
const MAX_POINTS: usize = 32_768;

fn mid(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new((a.x + b.x) / 2., (a.y + b.y) / 2.)
}
fn flat(a: Vec2, b: Vec2, p: Vec2) -> bool {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = js_hypot(dx, dy);
    if len == 0. {
        js_hypot(p.x - a.x, p.y - a.y) <= TOL
    } else {
        let dot = (p.x - a.x) * dx + (p.y - a.y) * dy;
        dot >= -TOL * len
            && dot <= len * len + TOL * len
            && ((p.x - a.x) * dy - (p.y - a.y) * dx).abs() <= TOL * len
    }
}
fn curve(a: Vec2, b: Vec2, c: Vec2, d: Vec2, depth: u8, out: &mut Vec<Vec2>) -> Result<(), String> {
    if out.len() >= MAX_POINTS {
        return Err("Yazı konturu güvenlik sınırını aşıyor.".into());
    }
    if flat(a, d, b) && flat(a, d, c) {
        out.push(d);
        return Ok(());
    }
    if depth == 16 {
        return Err("Yazı eğrisinin doğruluğu sağlanamadı.".into());
    }
    let ab = mid(a, b);
    let bc = mid(b, c);
    let cd = mid(c, d);
    let abc = mid(ab, bc);
    let bcd = mid(bc, cd);
    let m = mid(abc, bcd);
    curve(a, ab, abc, m, depth + 1, out)?;
    curve(m, bcd, cd, d, depth + 1, out)
}
fn polygons(ops: Vec<OutlineOp>) -> Result<Vec<Vec<Vec<[f64; 2]>>>, String> {
    let mut rings = Vec::<Vec<Vec2>>::new();
    let mut ring = Vec::new();
    let mut at = Vec2::new(0., 0.);
    let v = |p: [f64; 2]| Vec2::new(p[0], p[1]);
    for op in ops {
        match op {
            OutlineOp::Move(p) => {
                if !ring.is_empty() {
                    return Err("Açık yazı konturu.".into());
                }
                at = v(p);
                ring.push(at);
            }
            OutlineOp::Line(p) => {
                at = v(p);
                ring.push(at);
            }
            OutlineOp::Quad(c, p) => {
                let c = v(c);
                let p = v(p);
                let b = Vec2::new(at.x + 2. / 3. * (c.x - at.x), at.y + 2. / 3. * (c.y - at.y));
                let d = Vec2::new(p.x + 2. / 3. * (c.x - p.x), p.y + 2. / 3. * (c.y - p.y));
                curve(at, b, d, p, 0, &mut ring)?;
                at = p;
            }
            OutlineOp::Cubic(a, b, p) => {
                let p = v(p);
                curve(at, v(a), v(b), p, 0, &mut ring)?;
                at = p;
            }
            OutlineOp::Close => {
                if ring.len() > 1 && ring.first() == ring.last() {
                    ring.pop();
                }
                if ring.len() >= 3 {
                    rings.push(std::mem::take(&mut ring));
                } else {
                    ring.clear();
                }
            }
        }
    }
    if !ring.is_empty() {
        return Err("Kapanmamış yazı konturu.".into());
    }
    let points: Vec<_> = rings.iter().flatten().copied().collect();
    if points.len() > MAX_POINTS {
        return Err("Yazı konturu güvenlik sınırını aşıyor.".into());
    }
    let edges = rings
        .iter()
        .flat_map(|r| {
            r.iter()
                .zip(r.iter().cycle().skip(1))
                .take(r.len())
                .map(|(&a, &b)| Edge::Seg { a, b })
        })
        .collect();
    // Nonzero fill preserves counters, disconnected accents and overlapping
    // contours. Do not assume every contour after the first is a hole.
    let areas = overlay(
        &[Source {
            edges,
            points: Some(points),
            cut: None,
        }],
        Rule::First,
    );
    Ok(areas
        .into_iter()
        .map(|a| {
            std::iter::once(a.outer)
                .chain(a.holes)
                .map(|r| r.pts.into_iter().map(|p| [p.x, p.y]).collect())
                .collect()
        })
        .collect())
}

pub(super) fn prepare(
    def: &LineTypeDef,
    settings: &ProjectSettings,
    plot_scale: f64,
    resources: &kentos_contracts::ProjectStyles,
) -> Result<Option<Value>, String> {
    if !def.elements.iter().any(|e| e.is_complex()) {
        return Ok(None);
    }
    let unit = settings
        .drawing_unit
        .unwrap_or(kentos_contracts::DrawingUnit::M)
        .metres();
    let paper = |v: f64| match def.unit {
        PatternUnit::Paper => v,
        PatternUnit::Drawing => v * unit.0 / unit.1 * 1000. / plot_scale,
    };
    let mut at = 0.;
    let mut marks = Vec::new();
    for e in &def.elements {
        if let Some(shape) = &e.shape {
            let file = kentos_contracts::shape_resource::source_name(&shape.file);
            let id = kentos_contracts::shape_resource::id(file);
            let mut items = resources.items.iter().filter(|i| {
                kentos_contracts::shape_resource::is_resource(i)
                    && i["id"].as_str() == Some(id.as_str())
            });
            let item = items.next().ok_or_else(|| {
                format!(
                    "“{}” SHX şekli için “{}” kaynağı projeye eklenmemiş.",
                    shape.name, shape.file
                )
            })?;
            if items.next().is_some() {
                return Err(format!("“{file}” SHX kaynağı yinelenmiş."));
            }
            let bytes = kentos_contracts::shape_resource::read(item)?;
            let paths = kentos_geometry_core::text::shx::ShapeFile::read(&bytes)
                .and_then(|f| f.draw(&shape.name))
                .map_err(|e| format!("“{}” SHX şekli (“{file}”): {e}", shape.name))?;
            if paths.is_empty() {
                return Err(format!(
                    "“{}” SHX şeklinin çizilebilir konturu yok.",
                    shape.name
                ));
            }
            let paths: Vec<Vec<[f64; 2]>> = paths
                .into_iter()
                .map(|p| p.into_iter().map(|p| [p.x, p.y]).collect())
                .collect();
            let p = &shape.place;
            marks.push(json!({"at":at,"cap":0.,"height":paper(p.scale),"x":paper(p.x),"y":paper(p.y),
                "rotation":p.rotation,"absolute":p.absolute,"upright":p.upright,"polygons":[],"paths":paths}));
            continue;
        }
        let Some(text) = &e.text else {
            at += paper(e.length.abs());
            continue;
        };
        if text.text.len() > 512 || text.text.contains(['\n', '\r', '\t']) {
            return Err("Çizgi tipi yazısı en çok 512 UTF-8 bayt ve tek satır olmalı.".into());
        }
        let style = settings
            .text_styles
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(&text.style));
        if style.is_none()
            && !["standard", "standart"]
                .iter()
                .any(|s| text.style.eq_ignore_ascii_case(s))
        {
            return Err(format!("“{}” yazı stili bulunamadı.", text.style));
        }
        if let Some(file) = style.and_then(|s| s.font_file.as_deref())
            && fonts::resolve(file).1.substituted()
        {
            return Err(format!(
                "“{file}” yazı tipi çizilemiyor; kaynak değiştirilmedi."
            ));
        }
        let font = Font::from_id(
            style
                .map(|s| s.font)
                .or(settings.drawing_font)
                .unwrap_or(kentos_contracts::DrawingFont::Barlow)
                .id(),
        );
        let place = TextPlace {
            p: Vec2::new(0., 0.),
            text: &text.text,
            height: 1.,
            rotation: 0.,
            align: None,
            width_factor: style.and_then(|s| s.width_factor),
            box_width: None,
            line_spacing: None,
            runs: &[],
            flow: None,
            font: Some(font),
            bold: style.is_some_and(|s| s.bold),
            italic: style.is_some_and(|s| s.italic),
            lean: tan(style.and_then(|s| s.oblique).unwrap_or(0.).to_radians()),
            path: None,
        };
        let face =
            shaping::select_font(font.id(), if place.bold { 600 } else { 400 }, place.italic)
                .ok_or_else(|| "Çizgi tipi yazısının font yüzü bulunamadı.".to_owned())?;
        let cap = shaping::cap_em(face).map_err(|e| e.message(&text.text))?;
        let layout =
            placed::layout(&place, font, place.italic).map_err(|e| e.message(&text.text))?;
        let mut glyphs = Vec::new();
        let mut cost = 0;
        for glyph in &layout.glyphs {
            let p = polygons(glyph.outline().map_err(|e| e.message(&text.text))?)?;
            cost += p.iter().flatten().map(Vec::len).sum::<usize>();
            if cost > MAX_POINTS {
                return Err("Çizgi tipi yazısının toplam konturu güvenlik sınırını aşıyor.".into());
            }
            glyphs.extend(p);
        }
        let p = &text.place;
        let height = style
            .and_then(|s| s.height)
            .filter(|h| *h > 0.)
            .map_or_else(
                || paper(p.scale),
                |h| h * settings.plot_scale / plot_scale * p.scale,
            );
        marks.push(
            json!({"at":at,"cap":cap,"height":height,"x":paper(p.x),"y":paper(p.y),
            "rotation":p.rotation,"absolute":p.absolute,"upright":p.upright,"polygons":glyphs}),
        );
    }
    if !at.is_finite() || at <= 0. {
        return Err("Çizgi tipinin tekrar uzunluğu geçersiz.".into());
    }
    Ok(Some(json!({"name":def.name,"period":at,"marks":marks})))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        line_types::LineTypes,
        simple::{Stroke, line_symbol_of},
    };
    #[test]
    fn attached_shape_keeps_vector_units_offsets_and_all_scale_factors() {
        let resource: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../fixtures/kcad/v2/shape-resources.json"
        ))
        .unwrap();
        let resources: kentos_contracts::ProjectStyles =
            serde_json::from_value(resource["styles"].clone()).unwrap();
        let mut settings:ProjectSettings=serde_json::from_value(json!({"srid":0,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":50.,"drawingUnit":"mm","lineTypeScale":2.,
            "lineTypes":[{"name":"P","unit":"drawing","elements":[{"length":100.},{"length":-25.},
                {"length":0.,"shape":{"name":"FENCE","file":"C:\\CAD\\FENCE.SHX","scale":5.,"x":2.,"y":-3.,"rotation":45.}},{"length":-25.}]}]})).unwrap();
        for scale in [1., 20., 100.] {
            let types = LineTypes::at_scale_with_resources(&settings, &resources, scale);
            assert_eq!(types.loss("P"), None);
            let raw = types.apply(Some("P"), json!({"type":"simpleLine"}));
            let m = &raw["cadMarks"]["marks"][0];
            assert_eq!(m["cap"], 0.);
            assert!((m["height"].as_f64().unwrap() - 10. / scale).abs() < 1e-12);
            assert!((m["x"].as_f64().unwrap() - 4. / scale).abs() < 1e-12);
            assert!((m["y"].as_f64().unwrap() + 6. / scale).abs() < 1e-12);
            assert_eq!(
                m["paths"],
                json!([[[0., -1.], [0., 1.]], [[-1., 0.], [1., 0.]]])
            );
        }
        settings.line_types[0].elements[2]
            .shape
            .as_mut()
            .unwrap()
            .name = "absent".into();
        assert!(
            LineTypes::with_resources(&settings, &resources)
                .loss("P")
                .unwrap()
                .contains("absent")
        );
    }
    #[test]
    fn fixed_height_and_glyph_counters_survive_units_viewports_and_both_scale_factors() {
        let settings:ProjectSettings=serde_json::from_value(json!({"srid":0,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","workspace":"cad","drawingUnit":"mm","plotScale":50.,"lineTypeScale":2.,
            "textStyles":[{"id":"pipe","name":"Pipe","font":"barlow","height":4.,"widthFactor":0.8,"oblique":12.,"fontFile":"Barlow-Regular.ttf"}],
            "lineTypes":[{"name":"P","unit":"drawing","elements":[{"length":100.},{"length":-50.},
                {"length":0.,"text":{"text":"O","style":"PIPE","scale":2.,"x":10.,"y":-5.}},{"length":-50.}]}]})).unwrap();
        for scale in [20., 50., 100.] {
            let types = LineTypes::at_scale(&settings, scale);
            assert!(types.loss("P").is_none());
            let symbols = types.apply(
                Some("P"),
                line_symbol_of("ink", kentos_contracts::LineType::Continuous, 0.25),
            );
            let symbols = Stroke {
                line_type: None,
                scale: 3.,
                opacity: 1.,
            }
            .apply(symbols);
            let m = &symbols["layers"][0]["cadMarks"];
            let physical = |v: &Value| v.as_f64().unwrap() * scale / 1000.;
            assert!((physical(&m["period"]) - 1.2).abs() < 1e-12);
            let mark = &m["marks"][0];
            assert_eq!(mark["cap"], 0.7, "bundled Barlow nominal cap is 700/1000");
            assert!((physical(&mark["height"]) - 2.4).abs() < 1e-12);
            assert!((physical(&mark["at"]) - 0.9).abs() < 1e-12);
            assert!((physical(&mark["x"]) - 0.06).abs() < 1e-12);
            assert!((physical(&mark["y"]) + 0.03).abs() < 1e-12);
            let polygons = mark["polygons"].as_array().unwrap();
            assert_eq!(polygons.len(), 1, "one O outline");
            assert_eq!(
                polygons[0].as_array().unwrap().len(),
                2,
                "O counter remains a hole"
            );
        }
    }
    #[test]
    fn unavailable_styles_and_font_files_stay_explicit_losses() {
        let mut settings: ProjectSettings =
            serde_json::from_value(json!({"srid":0,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":50.,
            "lineTypes":[{"name":"P","unit":"paper","elements":[{"length":2.},{"length":-1.},
                {"length":0.,"text":{"text":"HW","style":"Unknown","scale":1.}}]}]}))
            .unwrap();
        assert!(
            LineTypes::of(&settings)
                .loss("P")
                .unwrap()
                .contains("Unknown")
        );
        settings.text_styles = serde_json::from_value(
            json!([{"id":"u","name":"Unknown","font":"barlow","fontFile":"missing.shx"}]),
        )
        .unwrap();
        assert!(
            LineTypes::of(&settings)
                .loss("P")
                .unwrap()
                .contains("missing.shx")
        );
    }
}
