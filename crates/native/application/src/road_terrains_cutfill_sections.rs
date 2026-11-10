//! The sections and the materials a cut and fill drawing writes (`road.terrains.cutfill`'s `sections` and `materials`,
//! docs/adr/4760; plan/map-cad.md MAP-15): the sections along a drawn axis (`kentos_road::volume_sections::along_axis`,
//! as `road.terrains.compare`'s) as their lines, each with its kilometre beyond its left end and its cut and fill areas
//! beyond its right end, along the section and kept upright; each material's name and volumes inside it (the core's
//! interior point, as the zones' labels); and with `table` a section volume table (kübaj cetveli: the kilometre, the
//! areas, the volumes from the section before and the totals) and a material table, under the zones' box.

use kentos_commands::{
    AnnotationKind, CutFillLabel, CutFillSection, CutFillSectionLabel, EntityBase, EntityGeometry, NewObject, TableAlign,
    TableEntity, TerrainZoneVolumes, TerrainsCutFilled, TextAlign, Vec2,
};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arrangement::Ring;
use kentos_geometry_core::ops::convert::points::interior_point;
use kentos_road::surface::Region;

/// A station as a kilometre, `k+mmm.cc`: the metres to the centimetre, half up.
pub(crate) fn km_text(station: f64) -> String {
    let cm = (station * 100.0 + 0.5).floor() as i64;
    format!("{}+{:03}.{:02}", cm / 100_000, (cm % 100_000) / 100, cm % 100)
}

/// A section's areas in words, to two decimals (m²).
pub(crate) fn area_text(cut: f64, fill: f64) -> String {
    format!("Kazı {cut:.2} m² · Dolgu {fill:.2} m²")
}

/// Each section's words: its kilometre at its left end, its areas at its right end; turned along the section from its
/// left end to its right, kept upright (a turn over 90° or down to −90° is turned half round), each aligned by its end
/// towards the section.
pub(crate) fn section_labels(sections: &[CutFillSection]) -> Vec<CutFillSectionLabel> {
    let mut out = Vec::with_capacity(2 * sections.len());
    for s in sections {
        let [l, r] = s.ends;
        let angle = (r[1] - l[1]).atan2(r[0] - l[0]).to_degrees();
        let flipped = angle > 90.0 || angle <= -90.0;
        let rotation = match (flipped, angle > 90.0) {
            (false, _) => angle,
            (true, true) => angle - 180.0,
            (true, false) => angle + 180.0,
        };
        let (km, areas) = if flipped { (TextAlign::MiddleLeft, TextAlign::MiddleRight) } else { (TextAlign::MiddleRight, TextAlign::MiddleLeft) };
        out.push(CutFillSectionLabel { at: l, text: s.km.clone(), rotation, align: km });
        out.push(CutFillSectionLabel { at: r, text: area_text(s.cut_area, s.fill_area), rotation, align: areas });
    }
    out
}

/// The section volume table's cells (kübaj cetveli, as Netcad's Enkesitlerden Hacim Hesabı writes it): the heading,
/// then each section: its kilometre, the distance from the section before, its cut and fill areas, the cut and fill
/// volumes from the section before and their running sums (none of the four for the first); numbers to two decimals.
pub(crate) fn section_table(sections: &[CutFillSection]) -> Vec<Vec<String>> {
    let two = |v: f64| format!("{v:.2}");
    let mut rows = vec![
        [
            "Km",
            "Ara uzaklık (m)",
            "Kazı alanı (m²)",
            "Dolgu alanı (m²)",
            "Kazı hacmi (m³)",
            "Dolgu hacmi (m³)",
            "Kümülatif kazı (m³)",
            "Kümülatif dolgu (m³)",
        ]
        .map(String::from)
        .to_vec(),
    ];
    let (mut cut, mut fill) = (0.0, 0.0);
    for (k, s) in sections.iter().enumerate() {
        let gap = k.checked_sub(1).map(|j| s.station - sections[j].station);
        cut += s.cut_volume.unwrap_or(0.0);
        fill += s.fill_volume.unwrap_or(0.0);
        let first = k == 0;
        rows.push(vec![
            s.km.clone(),
            gap.map(two).unwrap_or_default(),
            two(s.cut_area),
            two(s.fill_area),
            s.cut_volume.map(two).unwrap_or_default(),
            s.fill_volume.map(two).unwrap_or_default(),
            if first { String::new() } else { two(cut) },
            if first { String::new() } else { two(fill) },
        ]);
    }
    rows
}

/// A material's words: its name and its cut and fill volumes to two decimals (m³).
pub(crate) fn material_text(m: &TerrainZoneVolumes) -> String {
    format!("{}: kazı {:.2} m³, dolgu {:.2} m³", m.name, m.cut, m.fill)
}

/// A material's label: its words at the core's interior point of its boundary.
pub(crate) fn material_label(m: &TerrainZoneVolumes, region: &Region) -> Option<CutFillLabel> {
    let v = |r: &[[f64; 2]]| r.iter().map(|p| kentos_geometry_core::vec2::Vec2::new(p[0], p[1])).collect::<Vec<_>>();
    let shape = Shape::Polygon {
        pts: v(&region.outer),
        bulges: None,
        holes: (!region.holes.is_empty()).then(|| region.holes.iter().map(|h| Ring { pts: v(h), bulges: None }).collect()),
        parts: None,
    };
    interior_point(&shape).map(|p| CutFillLabel { at: [p.x, p.y], text: material_text(m) })
}

/// The material table's cells: the heading, then each material (its name, area, cut and fill, its factors and its
/// adjusted net); numbers to two decimals, the factors as given.
pub(crate) fn material_table(materials: &[TerrainZoneVolumes]) -> Vec<Vec<String>> {
    let two = |v: f64| format!("{v:.2}");
    let mut rows = vec![
        ["Malzeme", "Alan (m²)", "Kazı (m³)", "Dolgu (m³)", "Kazı katsayısı", "Dolgu katsayısı", "Katsayılı net (m³)"]
            .map(String::from)
            .to_vec(),
    ];
    for m in materials {
        rows.push(vec![
            m.name.clone(),
            two(m.area),
            two(m.cut),
            two(m.fill),
            format!("{}", m.adjusted.cut_factor),
            format!("{}", m.adjusted.fill_factor),
            two(m.adjusted.net),
        ]);
    }
    rows
}

fn bare(geometry: EntityGeometry) -> NewObject {
    NewObject {
        geometry,
        color: None,
        line_weight: None,
        line_type: None,
        line_pattern: None,
        attrs: None,
        label: None,
        label_of: None,
        label_scale: None,
        symbol: None,
        order: None,
        transparency: None,
    }
}

fn text(p: [f64; 2], words: &str, height: f64, rotation: f64, align: TextAlign) -> NewObject {
    bare(EntityGeometry::Text {
        layout: None,
        annotative: None,
        p: Vec2 { x: p[0], y: p[1] },
        text: words.to_owned(),
        height,
        rotation,
        align: Some(align),
        width_factor: None,
        mask: true,
        box_width: None,
        line_spacing: None,
        runs: Vec::new(),
        face: Default::default(),
        path: None,
    })
}

/// A table of `cells` whose top left corner is `p`, its first column left aligned and the others right, its columns
/// fitted as the Tablo tool fits them; and its height.
fn table(doc: &kentos_domain::Document, p: [f64; 2], cells: Vec<Vec<String>>) -> (NewObject, f64) {
    let h = doc.settings().annotation_height(AnnotationKind::Table);
    let columns = cells.first().map_or(0, Vec::len);
    let aligns: Vec<TableAlign> = (0..columns).map(|j| if j == 0 { TableAlign::Left } else { TableAlign::Right }).collect();
    let mut t = TableEntity {
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: String::new(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        p: Vec2 { x: p[0], y: p[1] },
        rotation: 0.0,
        height: h,
        rows: Vec::new(),
        columns: Vec::new(),
        cells: Vec::new(),
        merges: Vec::new(),
        aligns: Some(aligns),
        header: true,
        grid: None,
        frame: None,
        face: Default::default(),
        source: None,
        styles: Vec::new(),
        formulas: Vec::new(),
        fields: Vec::new(),
        breaks: None,
        table_style: None,
    };
    doc.take_rows(&mut t, cells);
    let tall = t.rows.iter().sum::<f64>();
    let object = bare(EntityGeometry::Table {
        p: t.p,
        rotation: t.rotation,
        height: t.height,
        rows: t.rows,
        columns: t.columns,
        cells: t.cells,
        merges: Vec::new(),
        aligns: t.aligns,
        header: true,
        grid: None,
        frame: None,
        face: Default::default(),
        source: None,
        styles: None,
        formulas: None,
        fields: None,
        table_style: None,
    });
    (object, tall)
}

/// The right edge the tables stand beyond (docs/adr/4760): the zones' box's, or the sections' ends' with room for their
/// longest words (a character a text height: wider than any of the drawing's fonts' letters) when that is farther.
pub(crate) fn right_edge(out: &TerrainsCutFilled, h: f64) -> f64 {
    let zones = out.zones.iter().flat_map(|z| z.ring.iter()).map(|p| p[0]).fold(f64::MIN, f64::max);
    let ends = out.sections.iter().flat_map(|s| s.ends.iter()).map(|p| p[0]).fold(f64::MIN, f64::max);
    let longest = out.section_labels.iter().map(|w| w.text.chars().count()).max().unwrap_or(0) as f64;
    if out.sections.is_empty() { zones } else { zones.max(ends + h / 2.0 + longest * h) }
}

/// What the sections and the materials add to the drawing, after the zones and their words: each section's line, its
/// two words (half a text's height beyond its ends), each material's words, then the section volume table and the
/// material table in the zones' table's column (two table heights right of `right_edge`), under it, one under the other.
pub(crate) fn objects(doc: &kentos_domain::Document, out: &TerrainsCutFilled) -> Vec<NewObject> {
    let h = doc.settings().annotation_height(AnnotationKind::Text);
    let mut objects = Vec::new();
    for s in &out.sections {
        let [l, r] = s.ends;
        objects.push(bare(EntityGeometry::Polyline {
            pts: vec![Vec2 { x: l[0], y: l[1] }, Vec2 { x: r[0], y: r[1] }],
            bulges: None,
            zs: None,
            ms: None,
            parts: None,
            closed: false,
            widths: None,
        }));
    }
    for (k, w) in out.section_labels.iter().enumerate() {
        // Beyond the end it stands at: away from the section's other end.
        let s = &out.sections[k / 2];
        let [l, r] = s.ends;
        let (from, to) = if k % 2 == 0 { (r, l) } else { (l, r) };
        let len = (to[0] - from[0]).hypot(to[1] - from[1]);
        let out_way = if len > 0.0 { [(to[0] - from[0]) / len, (to[1] - from[1]) / len] } else { [0.0, 0.0] };
        let at = [w.at[0] + out_way[0] * h / 2.0, w.at[1] + out_way[1] * h / 2.0];
        objects.push(text(at, &w.text, h, w.rotation, w.align));
    }
    for m in &out.material_labels {
        objects.push(text(m.at, &m.text, h, 0.0, TextAlign::MiddleCenter));
    }
    if out.section_table.is_empty() && out.material_table.is_empty() {
        return objects;
    }
    let t = doc.settings().annotation_height(AnnotationKind::Table);
    let x = right_edge(out, h) + 2.0 * t;
    let y1 = out.zones.iter().flat_map(|z| z.ring.iter()).map(|p| p[1]).fold(f64::MIN, f64::max);
    // Under the zones' quantity table when there is one: its rows are a row's height each.
    let zone_table = out.table.len() as f64 * kentos_geometry_core::ops::table::ROW * t;
    let mut top = if out.table.is_empty() { y1 } else { y1 - zone_table - 2.0 * t };
    for cells in [&out.section_table, &out.material_table] {
        if cells.is_empty() {
            continue;
        }
        let (object, tall) = table(doc, [x, top], cells.clone());
        objects.push(object);
        top -= tall + 2.0 * t;
    }
    objects
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_station_reads_as_a_kilometre() {
        assert_eq!(km_text(0.0), "0+000.00");
        assert_eq!(km_text(20.0), "0+020.00");
        assert_eq!(km_text(1234.5), "1+234.50");
        assert_eq!(km_text(999.996), "1+000.00");
        assert_eq!(km_text(62.831853), "0+062.83");
    }

    /// Along an eastward axis the section runs south: its words turn half round to stay upright and swap their sides.
    #[test]
    fn the_words_stay_upright() {
        let s = |l: [f64; 2], r: [f64; 2]| CutFillSection {
            station: 10.0,
            km: km_text(10.0),
            at: [0.0, 0.0],
            ends: [l, r],
            cut_area: 1.0,
            fill_area: 2.0,
            cut_volume: None,
            fill_volume: None,
        };
        let east = section_labels(&[s([0.0, 5.0], [0.0, -5.0])]);
        assert_eq!((east[0].rotation, east[0].align), (90.0, TextAlign::MiddleLeft));
        assert_eq!(east[1].align, TextAlign::MiddleRight);
        let north = section_labels(&[s([-5.0, 0.0], [5.0, 0.0])]);
        assert_eq!((north[0].rotation, north[0].align), (0.0, TextAlign::MiddleRight));
        assert_eq!(north[1].text, "Kazı 1.00 m² · Dolgu 2.00 m²");
    }
}
