//! One object's sections: the general rows, then its kind's own (wall.rs, space.rs, slab.rs, opening.rs,
//! arch.rs and the others), elevations, measures and where a value comes from.

use super::*;
use crate::properties::Coord;

/// A Kot row (docs/adr/0142): the elevations as `summary` says them, in
/// metres: a value or a range beside its unit, or `kot yok`; with vertices
/// that have none, a note under it names them.
pub(super) fn elevation_row(
    label: &'static str,
    summary: Elevations,
    f: &Format,
    editor: Option<Editor>,
) -> Vec<Row> {
    let (text, unit) = summary.shown(f);
    let row = match unit {
        Some(unit) => Row::figure(label, text).unit(unit),
        None => Row::text(label, text),
    };
    let mut rows = vec![row.editor(editor)];
    rows.extend(summary.note().map(Row::note));
    rows
}

pub(super) fn v(p: kentos_commands::Vec2) -> Vec2 {
    Vec2::new(p.x, p.y)
}

/// Degrees from radians with four decimals (the web's `(rad * 180 / π).toFixed(4)`).
pub(super) fn degrees(rad: f64) -> String {
    fixed((rad * 180.0) / std::f64::consts::PI, 4)
}

/// Degrees brought into [0, 360) with four decimals.
pub(super) fn turn(deg: f64) -> String {
    fixed(((deg % 360.0) + 360.0) % 360.0, 4)
}

/// A route's measures (docs/adr/0224 §6), read-only (the vertex table
/// writes them): a line's `M (başlangıç)` and `M (bitiş)`, a polyline's `M`
/// from its first measured vertex to its last, with a note when some
/// vertices have none; nothing for an object without a measure. The
/// numbers are the route's own, with the project's length decimals.
pub(super) fn measure_rows(e: &Entity, f: &Format) -> Vec<Row> {
    let t = |m: f64| fixed(m, f.length_decimals);
    let shown = |m: Option<f64>| m.map_or_else(|| "ölçü yok".to_owned(), t);
    match e {
        Entity::Line(l) if l.ma.is_some() || l.mb.is_some() => vec![
            Row::figure("M (başlangıç)", shown(l.ma)),
            Row::figure("M (bitiş)", shown(l.mb)),
        ],
        Entity::Polyline(p) => {
            let own = p.ms.clone().unwrap_or_else(|| vec![None; p.pts.len()]);
            let all: Vec<Option<f64>> = own
                .into_iter()
                .chain(
                    p.parts
                        .iter()
                        .flatten()
                        .flat_map(|q| q.ms.clone().unwrap_or_else(|| vec![None; q.pts.len()])),
                )
                .collect();
            let ms: Vec<f64> = all.iter().flatten().copied().collect();
            let (Some(first), Some(last)) = (ms.first(), ms.last()) else {
                return Vec::new();
            };
            let mut rows = vec![Row::figure("M", format!("{} → {}", t(*first), t(*last)))];
            if ms.len() < all.len() {
                rows.push(Row::note("(bazı köşeler ölçüsüz)"));
            }
            rows
        }
        _ => Vec::new(),
    }
}

pub(super) fn yes_no(on: bool) -> &'static str {
    if on { "Evet" } else { "Hayır" }
}

/// A polyline's one width for every edge, 0 without widths; none when its edges' differ
/// (docs/adr/0258 §2).
pub(super) fn constant_width(line: &kentos_commands::PathEntity) -> Option<f64> {
    let Some(widths) = &line.widths else {
        return Some(0.0);
    };
    let n = line.pts.len();
    let edges = if line.closed { n } else { n.saturating_sub(1) };
    let first = widths.first()?[0];
    widths
        .get(..edges)?
        .iter()
        .all(|[a, b]| *a == first && *b == first)
        .then_some(first)
}

/// `3B uzunluk` of a line or a polyline, `3B çevre` of an area, when every
/// vertex has an elevation (docs/adr/0142): the length in space beside the
/// plan one, which stays the measure of record.
pub(super) fn space_length(e: &Entity, f: &Format) -> Option<Row> {
    let (label, length) = elevation::space_length(e)?;
    Some(Row::figure(label, f.length_bare(length)).unit(f.length_unit_label()))
}

/// The “Bu değer nereden geliyor?” rows (docs/adr/1100 §5): each part's value and the level it comes from, then
/// quiet lines for the levels under it (layer, standard, block), the viewports that draw it otherwise and the reach
/// of each reset, read before anything is reset.
pub(super) fn origin_rows(
    parts: &[kentos_native_application::appearance_origin::Explained],
) -> Vec<Row> {
    let mut rows = Vec::new();
    for p in parts {
        rows.push(Row::text(p.label, format!("{} · {}", p.value, p.origin.label())));
        for link in p.chain.iter().filter(|l| !l.wins && l.value != "—") {
            rows.push(Row::note(&format!("{}: {} · {}", link.origin.label(), link.value, link.note)));
        }
        for v in &p.viewports {
            let mut row = Row::note(&format!("Pencerede başka: {v}"));
            row.warn = true;
            rows.push(row);
        }
        for v in &p.printed {
            rows.push(Row::note(&format!("Baskıda: {v}")));
        }
        for r in &p.resets {
            rows.push(Row::note(&format!("Sıfırla → {} · {} ({})", r.after, r.scope, r.count)));
        }
    }
    rows
}

/// One object's sections: Genel, Geometri, an insert's Blok öznitelikleri, and Öznitelik bilgileri when it has other attributes.
/// The rows a building element's own section starts with, in this order (docs/adr/8005, plan ADV-09): what makes it
/// the element it is, then where it stands.
pub(super) const BUILDING_MAIN: [&str; 10] = [
    "Numara",
    "Ad",
    "Tür",
    "Kalınlık",
    "Genişlik",
    "Stil",
    "Yükseklik",
    "Hiza",
    "Kat",
    "Tadilat durumu",
];

/// A building element's own section comes first, named for it, its main rows at its top. A wall's Kalınlık, Stil,
/// Yükseklik, Hiza, Kat and Tadilat durumu were under the eight rows every object has (Tür, Katman, Renk, Çizgi kalınlığı …) and
/// further down their own section, below the joins: at 1100 × 650 and 18 px only Tür showed (plan S18). The general
/// rows follow, as they were.
pub(super) fn building_first(e: &Entity, sections: &mut Vec<Section>) {
    if !matches!(
        e,
        Entity::Wall(_)
            | Entity::Opening(_)
            | Entity::Space(_)
            | Entity::Slab(_)
            | Entity::Stair(_)
            | Entity::Ramp(_)
            | Entity::Beam(_)
            | Entity::Railing(_)
            | Entity::Column(_)
            | Entity::Grid(_)
    ) {
        return;
    }
    let Some(i) = sections.iter().position(|s| s.id == "geometry") else { return };
    let mut own = sections.remove(i);
    own.title = kind_title(e.kind()).into();
    // Each row with the notes under it, so a note keeps its row.
    let mut groups: Vec<Vec<Row>> = Vec::new();
    for row in std::mem::take(&mut own.rows) {
        match groups.last_mut() {
            Some(group) if row.note => group.push(row),
            _ => groups.push(vec![row]),
        }
    }
    let rank = |g: &Vec<Row>| BUILDING_MAIN.iter().position(|m| g[0].label == *m).unwrap_or(BUILDING_MAIN.len());
    groups.sort_by_key(rank);
    own.rows = groups.into_iter().flatten().collect();
    sections.insert(0, own);
}

pub(super) fn entity_sections(doc: &Document, e: &Entity, shown: Shown) -> Vec<Section> {
    let base = e.base();
    let slot = Slot(base.id);
    let layers = doc.model.layers();
    let locked = layers.is_locked(&base.layer_id);
    let ids = [slot];
    let edit = |editor: Editor| (!locked).then_some(editor);
    let color = Some(base.color.as_deref());
    let symbol = Some(base.symbol.as_deref());

    let mut general = vec![
        Row::text("Tür", kind_title(e.kind())),
        Row::text(
            "Katman",
            if locked {
                format!("{} (kilitli)", layer_path(layers, &base.layer_id))
            } else {
                String::new()
            },
        )
        .editor(edit(layer_editor(doc, &ids, Some(&base.layer_id)))),
    ];
    // A record is not drawn (docs/adr/0410 §4): no colour, transparency or symbol.
    let drawn = e.has_geometry();
    if drawn {
        general.push(Row::text("Renk", color_text(color)).editor(edit(color_editor(&ids, color))));
    }
    if e.draws_lines() {
        let weight = Some(base.line_weight);
        general.push(
            Row::text("Çizgi kalınlığı", weight_text(weight)).editor(edit(weight_editor(&ids, weight))),
        );
        // Its own line type, its scale and transparency (docs/adr/0260).
        general.push(
            line_type_row(
                doc,
                &ids,
                Some(base.look.line_type),
                Some(base.look.line_pattern.as_deref()),
            )
            .editor_if(!locked),
        );
        general.push(line_type_scale_row(&ids, Some(base.look.line_type_scale)).editor_if(!locked));
    }
    if drawn {
        general.push(transparency_row(&ids, Some(base.look.transparency)).editor_if(!locked));
    }
    // An area's mask (docs/adr/0310 §5), where the program has masks.
    if shown.mask && matches!(e, Entity::Polygon(_)) {
        general.push(wipeout_row(&ids, Some(base.look.wipeout)).editor_if(!locked));
    }
    if drawn && takes_symbol(e) {
        general.push(
            Row::text("Sembol", symbol_text(doc, symbol)).editor(edit(symbol_editor(doc, symbol))),
        );
    }

    let (area_of, length_of) = measures(e);
    let kit = Kit {
        doc,
        slot,
        locked,
        e,
        f: Format::of(doc.settings()),
        decimals: doc.settings().area_decimals as usize,
        area_of,
        length_of,
    };
    let f = kit.f;
    let len = |label: &'static str, value: f64| kit.len(label, value);
    let metres = |label: &'static str, value: f64| kit.metres(label, value);
    let area = |m2: f64| kit.area(m2);
    // A direction as the project's type reads it: a semt, or a CAD project's angle (docs/adr/0165 §4).
    let bearing = |a: Vec2, b: Vec2| {
        Row::figure(f.direction_name(), f.direction_bare(bearing_grad(a, b)))
            .unit(f.angle_unit_label())
    };
    let number = |field: Field| kit.number(field);

    let mut geo: Vec<Row> = Vec::new();
    match e {
        Entity::Point(p) => {
            // East and north as the project's type names them (docs/adr/0165 §4).
            let (east, north) = match f.axes {
                kentos_interaction::Axes::Cad => ("X (sağa)", "Y (yukarı)"),
                kentos_interaction::Axes::Gis => ("Y (sağa)", "X (yukarı)"),
            };
            geo.push(metres(east, p.p.x).editor(number(Field::PointX(slot))));
            geo.push(metres(north, p.p.y).editor(number(Field::PointY(slot))));
            match p.parts.as_deref().filter(|ps| !ps.is_empty()) {
                // A multi-point object: how many points, and their elevations as one row (docs/adr/0174).
                Some(parts) => {
                    geo.push(Row::figure("Nokta sayısı", (parts.len() + 1).to_string()));
                    geo.extend(elevation_row(
                        "Kot",
                        Elevations::of_object(e),
                        &f,
                        number(Field::Elevation(slot, Spot::All)),
                    ));
                }
                None => {
                    if let Some(z) = p.z {
                        geo.push(metres("Z (kot)", z));
                    }
                }
            }
        }
        Entity::Line(l) => {
            // Each end's elevation follows its coordinates (docs/adr/0142).
            // Each end typed as a point's coordinates are (a whole number, its unit if any).
            geo.extend([
                len("Başlangıç Y", l.a.x).editor(number(Field::Geometry(slot, Coord::StartEast))),
                len("Başlangıç X", l.a.y).editor(number(Field::Geometry(slot, Coord::StartNorth))),
            ]);
            geo.extend(elevation_row(
                "Kot (başlangıç)",
                Elevations::of(&[l.za]),
                &f,
                number(Field::Elevation(slot, Spot::Start)),
            ));
            geo.extend([
                len("Bitiş Y", l.b.x).editor(number(Field::Geometry(slot, Coord::EndEast))),
                len("Bitiş X", l.b.y).editor(number(Field::Geometry(slot, Coord::EndNorth))),
            ]);
            geo.extend(elevation_row(
                "Kot (bitiş)",
                Elevations::of(&[l.zb]),
                &f,
                number(Field::Elevation(slot, Spot::End)),
            ));
            geo.extend(measure_rows(e, &f));
            geo.push(metres("Uzunluk", dist(v(l.a), v(l.b))));
            geo.extend(space_length(e, &f));
            geo.push(bearing(v(l.a), v(l.b)));
        }
        Entity::Polyline(p) | Entity::Polygon(p) => {
            let polygon = matches!(e, Entity::Polygon(_));
            if !polygon
                && ((shown.separators
                    && doc.settings().workspace == Some(kentos_commands::Workspace::Cad))
                    || p.space_separator.is_some())
            {
                geo.extend(arch::separator_rows(doc,slot,p,locked));
            }
            // A multi-part area's corners, parts and holes are every part's (docs/adr/0143).
            let parts = p.parts.as_deref().unwrap_or(&[]);
            let corners = p.pts.len() + parts.iter().map(|q| q.pts.len()).sum::<usize>();
            geo.push(Row::figure("Köşe sayısı", corners.to_string()));
            if !parts.is_empty() {
                geo.push(Row::figure("Parça sayısı", (parts.len() + 1).to_string()));
            }
            geo.extend(elevation_row(
                "Kot",
                Elevations::of_object(e),
                &f,
                if p.space_separator.is_some(){None}else{number(Field::Elevation(slot, Spot::All))},
            ));
            geo.extend(measure_rows(e, &f));
            geo.push(metres(
                if polygon { "Çevre" } else { "Uzunluk" },
                length_of.unwrap_or(0.0),
            ));
            geo.extend(space_length(e, &f));
            // A one-part polyline's Kapalı and Genişlik; a closed one encloses an area, shown,
            // while it has none of its own (docs/adr/0258).
            if let Entity::Polyline(line) = e
                && parts.is_empty()
            {
                let closings = Editor::Select {
                    text: yes_no(line.closed).to_owned(),
                    swatch: None,
                    icon: None,
                    items: [true, false]
                        .into_iter()
                        .map(|on| Choice::Pick {
                            label: yes_no(on).to_owned(),
                            swatch: None,
                            icon: None,
                            wide: false,
                            chosen: line.closed == on,
                            enabled: true,
                            message: Message::Properties(Event::PolylineClosed(vec![slot], on)),
                        })
                        .collect(),
                };
                geo.push(Row::text("Kapalı", yes_no(line.closed)).editor(edit(closings)));
                // Its widths where the program has a polyline's widths (docs/adr/0802).
                if shown.widths {
                    let width = if line.space_separator.is_some() {
                        None
                    } else {
                        number(Field::PolylineWidth(slot))
                    };
                    geo.push(match constant_width(line) {
                        Some(w) => metres("Genişlik", w).editor(width),
                        None => Row::text("Genişlik", "Değişken").editor(width),
                    });
                }
                if line.closed {
                    geo.extend(area(kentos_interaction::enclosed(e).unwrap_or(0.0)));
                }
            }
            if polygon {
                // Net area: the holes (adalar) are taken out already.
                let holes = p.holes.iter().flatten().count()
                    + parts
                        .iter()
                        .map(|q| q.holes.iter().flatten().count())
                        .sum::<usize>();
                if holes > 0 {
                    geo.push(Row::figure("Ada (delik)", holes.to_string()));
                }
                geo.extend(area(area_of.unwrap_or(0.0)));
            }
        }
        Entity::Circle(c) => {
            geo.extend([
                len("Merkez Y", c.c.x).editor(number(Field::Geometry(slot, Coord::CenterEast))),
                len("Merkez X", c.c.y).editor(number(Field::Geometry(slot, Coord::CenterNorth))),
                metres("Yarıçap", c.r).editor(number(Field::Geometry(slot, Coord::Radius))),
            ]);
            geo.extend(area(area_of.unwrap_or(0.0)));
        }
        Entity::Arc(a) => {
            geo.extend([
                len("Merkez Y", a.c.x),
                len("Merkez X", a.c.y),
                metres("Yarıçap", a.r),
                Row::figure("Başlangıç açısı", degrees(a.a0)).unit("°"),
                Row::figure("Bitiş açısı", degrees(a.a1)).unit("°"),
                Row::figure("Yay açısı", degrees(arc_sweep(a.a0, a.a1))).unit("°"),
                metres("Yay uzunluğu", length_of.unwrap_or(0.0)),
            ]);
        }
        Entity::Ellipse(el) => {
            let a = el.major.x.hypot(el.major.y);
            geo.extend([
                len("Merkez Y", el.c.x),
                len("Merkez X", el.c.y),
                metres("Büyük yarı eksen", a),
                metres("Küçük yarı eksen", a * el.ratio),
                Row::figure(
                    "Eksen açısı",
                    turn(angle_deg(Vec2::new(0.0, 0.0), v(el.major))),
                )
                .unit("°"),
            ]);
            if full_ellipse(el) {
                geo.push(metres("Çevre", length_of.unwrap_or(0.0)));
                geo.extend(area(area_of.unwrap_or(0.0)));
            } else {
                let param = |rad: f64| turn((rad * 180.0) / std::f64::consts::PI);
                geo.extend([
                    Row::figure("Başlangıç parametresi", param(el.t0)).unit("°"),
                    Row::figure("Bitiş parametresi", param(el.t1)).unit("°"),
                    metres("Yay uzunluğu", length_of.unwrap_or(0.0)),
                ]);
            }
        }
        Entity::Xline(x) | Entity::Ray(x) => {
            let ray = matches!(e, Entity::Ray(_));
            let p = v(x.p);
            geo.extend([
                len(
                    if ray {
                        "Başlangıç Y"
                    } else {
                        "Geçtiği nokta Y"
                    },
                    x.p.x,
                ),
                len(
                    if ray {
                        "Başlangıç X"
                    } else {
                        "Geçtiği nokta X"
                    },
                    x.p.y,
                ),
                Row::figure("Doğrultu", turn(angle_deg(Vec2::new(0.0, 0.0), v(x.dir)))).unit("°"),
                bearing(p, Vec2::new(p.x + x.dir.x, p.y + x.dir.y)),
            ]);
        }
        Entity::Spline(s) => {
            geo.extend([
                Row::figure("Nokta sayısı", s.pts.len().to_string()),
                Row::text("Kapalı", if s.closed { "Evet" } else { "Hayır" }),
                metres("Uzunluk", length_of.unwrap_or(0.0)),
            ]);
        }
        // A wall (docs/adr/0339 §6): its width, justification, offset, style, ends, height, level and phase, each
        // written through `cad.walls.edit`; its length along its path and its body's area.
        Entity::Wall(w) => geo.extend(wall::rows(&kit, w)),
        // A grid, column, stair or ramp (docs/adr/0343 §5): their rows below.
        Entity::Grid(_)
        | Entity::Column(_)
        | Entity::Stair(_)
        | Entity::Ramp(_)
        | Entity::Beam(_)
        | Entity::Railing(_)
        | Entity::Tag(_)
        | Entity::ViewMark(_) => {
            geo.extend(arch::rows(doc, slot, e, locked, &f));
        }
        // A roof (docs/adr/8044 §1): what it is by its slopes, its slopes and gable ends, its eaves' and ridge's heights
        // (PAİY 40/4: the ridge at most 5.00 m over the eaves), its overhang (PAİY 42: 1.00 m), its covering, its storey.
        Entity::Roof(r) => {
            let x = &r.roof;
            let settings = doc.model.settings();
            geo.push(Row::text("Tür", x.kind_name()));
            let mut slopes: Vec<f64> = x.slopes.iter().flatten().copied().collect();
            slopes.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
            if !slopes.is_empty() {
                let words: Vec<String> = slopes
                    .iter()
                    .map(|s| kentos_geometry_core::geom::roof::slope_words(*s))
                    .collect();
                geo.push(Row::text("Eğim", words.join(" / ")));
            }
            let gables = x.slopes.iter().filter(|s| s.is_none()).count();
            if gables > 0 && gables < x.slopes.len() {
                geo.push(Row::text("Kalkan", format!("{gables} kenar")));
            }
            let eaves =
                kentos_native_application::geometry::storey_top(settings, x.arch.level.as_deref())
                    .unwrap_or(0.0)
                    + x.offset;
            let core: Vec<kentos_geometry_core::Vec2> = x
                .pts
                .iter()
                .map(|q| kentos_geometry_core::Vec2::new(q.x, q.y))
                .collect();
            let spec = kentos_geometry_core::geom::roof::RoofSpec {
                slopes: x.slopes.clone(),
                base: eaves,
                ..Default::default()
            };
            geo.push(Row::text(
                "Saçak kotu",
                kentos_geometry_core::geom::roof::level_words(eaves),
            ));
            match kentos_geometry_core::geom::roof::ridge_rise(&core, &spec) {
                Ok(rise) => {
                    geo.push(Row::text(
                        "Mahya kotu",
                        kentos_geometry_core::geom::roof::level_words(eaves + rise),
                    ));
                    geo.push(metres("Mahya yüksekliği", rise));
                }
                Err(kentos_geometry_core::geom::roof::Problem::Concave) => {
                    geo.push(Row::text(
                        "Mahya kotu",
                        "İçe dönen köşeli saçak henüz hesaplanmıyor",
                    ));
                }
                Err(_) => {}
            }
            if let Some(o) = x.overhang {
                geo.push(metres("Saçak", o));
            }
            geo.push(Row::text("Kaplama", x.material_name()));
            // Its gutter and snow guard (docs/adr/8044 §8; Sivas İmar Yönetmeliği 42/5): along its sloping eaves.
            for (label, v) in [("Oluk", x.gutter), ("Kar siperliği", x.snow_guard)] {
                geo.push(match v {
                    Some(v) => metres(label, v),
                    None => Row::text(label, "Yok"),
                });
            }
            // Its chimneys, vents and windows (docs/adr/8044 §6), numbered within their kind when more than one: each
            // one's size and a chimney's or a vent's top over ±0.00.
            if let kentos_geometry_core::entity::Shape::Roof { pts, roof: spec } =
                kentos_native_application::geometry::roof_shape(r, settings)
            {
                for (k, (part, core)) in x.parts.iter().zip(&spec.parts).enumerate() {
                    let of_kind = x.parts.iter().filter(|p| p.kind == part.kind).count();
                    let nth = x.parts[..=k].iter().filter(|p| p.kind == part.kind).count();
                    let label = if of_kind > 1 {
                        format!("{} {nth}", part.kind.label())
                    } else {
                        part.kind.label().to_owned()
                    };
                    let size = if part.kind == kentos_commands::RoofPartKind::Downpipe {
                        format!("Ø{}", f.length(part.width))
                    } else {
                        format!("{} × {}", f.length_bare(part.width), f.length(part.depth))
                    };
                    let text = match kentos_geometry_core::geom::roof::part_top(&pts, &spec, core) {
                        Some(top) => format!(
                            "{size} · tepesi {}",
                            kentos_geometry_core::geom::roof::level_words(top)
                        ),
                        None => size,
                    };
                    geo.push(Row::text(label, text));
                }
            }
            geo.push(metres("Çevre", length_of.unwrap_or(0.0)));
            geo.extend(area(area_of.unwrap_or(0.0)));
        }
        // A space (docs/adr/0342 §5): its number, name, usage, method, level, phase, height, finishes, measures and
        // boundary, each written through `cad.spaces.edit`; why it lost its walls.
        Entity::Space(sp) => geo.extend(space::rows(&kit, sp)),
        // A slab (docs/adr/0722 §6): its role, thickness, top's offset and elevation, edge and its width, material,
        // level, phase and whether Metraj sums it, each written through `cad.slabs.edit`; its area, perimeter, volume
        // and edge's length.
        Entity::Slab(sl) => geo.extend(slab::rows(&kit, sl)),
        // An opening (docs/adr/0341 §5): its form, type, width, place along its wall, side, hinge, leaves, operation,
        // swing, frame, sill, head and number, each written through `cad.openings.edit`; its wall, or why it is in none.
        Entity::Opening(o) => geo.extend(opening::rows(&kit, o)),
        // A NURBS curve (docs/adr/0259 §6): its degree, control and fit points, whether it is rational
        // and closed, its length and, closed, the area it encloses.
        Entity::Nurbs(n) => {
            geo.extend([
                Row::figure("Derece", n.degree.to_string()),
                Row::figure("Denetim noktası", n.ctrl.len().to_string()),
            ]);
            if let Some(fit) = &n.fit {
                geo.push(Row::figure("Geçiş noktası", fit.len().to_string()));
            }
            geo.extend([
                Row::text(
                    "Ağırlıklı",
                    if n.weights.is_some() {
                        "Evet"
                    } else {
                        "Hayır"
                    },
                ),
                Row::text("Kapalı", if n.closed { "Evet" } else { "Hayır" }),
                metres("Uzunluk", length_of.unwrap_or(0.0)),
            ]);
            if n.closed {
                geo.extend(area(kentos_interaction::enclosed(e).unwrap_or(0.0)));
            }
        }
        Entity::Dimension(d) => {
            let set_member = doc
                .model
                .uid(slot)
                .and_then(|uid| doc.model.dimension_set_of_item(uid));
            let style = d.style.unwrap_or(DimensionStyle::Aligned);
            geo.push(Row::text(
                "Tür",
                match style {
                    DimensionStyle::Aligned => "Hizalı",
                    DimensionStyle::Linear => "Doğrusal",
                    DimensionStyle::Angular => "Açı",
                    DimensionStyle::Radius => "Yarıçap",
                    DimensionStyle::Diameter => "Çap",
                    DimensionStyle::Ordinate => "Koordinat",
                    DimensionStyle::ArcLength => "Yay uzunluğu",
                    DimensionStyle::Jogged => "Kırıklı yarıçap",
                    DimensionStyle::Azimuth => "Semt",
                    DimensionStyle::Slope => "Eğim",
                    DimensionStyle::CenterMark => "Merkez işareti",
                    DimensionStyle::CenterLine => "Eksen çizgisi",
                    DimensionStyle::Elevation => "Kot",
                },
            ));
            // What it measured, by its unit (docs/adr/0147 §2).
            match dimension_layout(e) {
                Some(l) if l.unit == "angle" => geo.push(
                    Row::figure(
                        if style == DimensionStyle::Azimuth {
                            "Ölçülen semt"
                        } else {
                            "Ölçülen açı"
                        },
                        f.angle_bare(l.value),
                    )
                    .unit(f.angle_unit_label()),
                ),
                Some(l) if l.unit == "percent" => {
                    geo.push(Row::figure("Ölçülen eğim", f.percent(l.value)).unit("%"))
                }
                // docs/adr/0322: a mark's curve or axis, a level's elevation.
                Some(l) if l.unit == "none" => geo.push(metres(
                    if style == DimensionStyle::CenterMark {
                        "Eğrinin yarıçapı"
                    } else {
                        "Eksenin uzunluğu"
                    },
                    l.value,
                )),
                // A Kot bound to a level reads it (docs/adr/0721 §2): its value is shown only.
                Some(l) if l.unit == "level" => geo.push(metres("Kot", l.value).editor(if d.level.is_some() {
                    None
                } else {
                    number(Field::DimensionZa(vec![slot]))
                })),
                Some(l) if l.unit == "coordinate" => geo.push(metres(
                    if l.prefix == "Y=" {
                        "Ölçülen Y"
                    } else {
                        "Ölçülen X"
                    },
                    l.value,
                )),
                Some(l) => geo.push(metres(
                    match style {
                        DimensionStyle::Diameter => "Ölçülen çap",
                        DimensionStyle::Radius | DimensionStyle::Jogged => "Ölçülen yarıçap",
                        _ => "Ölçülen uzunluk",
                    },
                    l.value,
                )),
                None => {}
            }
            if style == DimensionStyle::Aligned {
                geo.push(bearing(v(d.a), v(d.b)));
            }
            // An ordinate and a level have no offset; a jogged radius's is where its jog is.
            if !matches!(style, DimensionStyle::Ordinate | DimensionStyle::Elevation) {
                geo.push(
                    metres(
                        match style {
                            DimensionStyle::Angular => "Yay yarıçapı",
                            DimensionStyle::Radius | DimensionStyle::Diameter => "Dışa uzantı",
                            DimensionStyle::Jogged => "Kırık uzaklığı",
                            DimensionStyle::CenterMark | DimensionStyle::CenterLine => "Uzantı",
                            _ => {
                                if set_member.is_some() {
                                    "Sıra uzaklığı"
                                } else {
                                    "Ötelenme"
                                }
                            }
                        },
                        set_member
                            .and_then(|(set, index)| {
                                doc.model
                                    .dimension_set_row_offset(set, set.members[index].row)
                                    .ok()
                            })
                            .unwrap_or(d.offset),
                    )
                    .editor(number(Field::DimensionOffset(slot))),
                );
            }
            // Ölçü stili, a CAD project's (docs/adr/0183 §6).
            geo.extend(style_rows(
                doc.model.settings(),
                false,
                &[d.look.dim_style.as_ref()],
                &ids,
                locked,
            ));
            geo.extend(override_rows(&[&d.look.overrides], &ids, locked));
            geo.extend(scale_rows(&[e], &ids, locked));
            geo.extend([
                metres("Yazı yüksekliği", d.height).editor(number(Field::DimensionHeight(slot))),
                Row::text("Yazı", d.text.clone().unwrap_or_default())
                    .editor(edit(Editor::Text(Field::DimensionText(slot)))),
            ]);
            // Zemin, an ordinate's axis, a slope's elevations, an arc length's radius and angle (docs/adr/0147 §7).
            let levels = doc.model.settings().arch.as_ref().map_or(&[][..], |a| a.levels.as_slice());
            let mut own = dimension::rows(&[d], &ids, locked, &f, levels);
            if set_member.is_some() {
                // Letting go is explicit for a recipe; freeing just one reference would be recreated.
                if let Some(relation) = own.iter_mut().find(|row| row.label == "İlişki") {
                    relation.editor = Some(Editor::Select {
                        text: relation.value.clone(),
                        swatch: None,
                        icon: None,
                        items: vec![Choice::Action {
                            label: "Diziyi bağımsız ölçülere dönüştür",
                            icon: "dimension",
                            message: (!locked).then_some(Message::Properties(Event::DimensionSet(
                                slot, "detach",
                            ))),
                        }],
                    });
                }
                geo.extend(dimension_set::rows(doc, slot, locked));
            }
            // İlişki right under its kind: whether the dimension follows its objects (docs/adr/0249 §3).
            if let Some(i) = own.iter().position(|r| r.label == "İlişki") {
                geo.insert(1.min(geo.len()), own.remove(i));
            }
            // A Kot's Kat and Fark right under its value: the level it reads (docs/adr/0721 §2).
            if let Some(at) = geo.iter().position(|r| r.label == "Kot") {
                for (k, label) in ["Kat", "Fark"].into_iter().enumerate() {
                    if let Some(i) = own.iter().position(|r| r.label == label) {
                        geo.insert(at + 1 + k, own.remove(i));
                    }
                }
            }
            geo.extend(own);
        }
        Entity::Hatch(h) => {
            geo.extend(hatch_rows(
                h,
                slot,
                locked,
                doc.model.settings().plot_scale,
                &f,
            ));
            geo.extend(scale_rows(&[e], &ids, locked));
            geo.extend(area(area_of.unwrap_or(0.0)));
        }
        Entity::Text(t) => {
            // Yazı stili, a CAD project's (docs/adr/0183 §6).
            geo.extend(style_rows(
                doc.model.settings(),
                true,
                &[t.face.text_style.as_ref()],
                &ids,
                locked,
            ));
            geo.extend(override_rows(&[&t.face.overrides], &ids, locked));
            geo.extend(scale_rows(&[e], &ids, locked));
            geo.extend([
                // A multi-line text's lines and formats are its editor's (a double click,
                // docs/adr/0182 §4): here they only show, ⏎ its breaks.
                if crate::paragraph_editor::is_paragraph(t) {
                    Row::text("Metin", t.text.replace('\n', " ⏎ "))
                } else {
                    Row::text("Metin", t.text.clone()).editor(edit(Editor::Text(Field::Text(slot))))
                },
                metres("Yükseklik", t.height).editor(number(Field::TextHeight(slot))),
                Row::figure("Açı", fixed(t.rotation, 2))
                    .unit("°")
                    .editor(number(Field::TextAngle(slot))),
            ]);
            // Hiza, Genişlik çarpanı and Zemin (docs/adr/0145 §6).
            geo.extend(text_rows(&[t], &ids, locked));
            // Bağlı nesne (docs/adr/0175 §4).
            geo.extend(link_rows(&doc.model, &[t], &ids, locked));
            // Alanlar (docs/adr/0325 §4).
            geo.extend(text_field_rows(&doc.model, t, slot, locked));
            // Along a curve, its curve's length (docs/adr/0196 §4).
            if let Some(path) = &t.path {
                let curve = kentos_native_application::geometry::core_curve(path);
                let p = kentos_geometry_core::Vec2::new(t.p.x, t.p.y);
                geo.push(metres("Eğri", curve.length(p, t.rotation)));
            }
            geo.extend([len("Konum Y", t.p.x), len("Konum X", t.p.y)]);
        }
        // Its style, size, turn and source (docs/adr/0184 §6); its cells are its editor's (a double click).
        Entity::Table(t) => {
            geo.extend(style_rows(
                doc.model.settings(),
                true,
                &[t.face.text_style.as_ref()],
                &ids,
                locked,
            ));
            geo.extend(table_style_row(doc.model.settings(), t, &ids, locked));
            geo.extend([
                Row::figure("Satır sayısı", t.rows.len().to_string()),
                Row::figure("Sütun sayısı", t.columns.len().to_string()),
                metres("Yazı yüksekliği", t.height).editor(number(Field::TextHeight(slot))),
                Row::figure("Açı", fixed(t.rotation, 2))
                    .unit("°")
                    .editor(number(Field::TextAngle(slot))),
                metres("Genişlik", t.columns.iter().sum()),
                metres("Derinlik", t.rows.iter().sum()),
                Row::text("Kaynak", crate::tables::source_words(t)),
                len("Konum Y", t.p.x),
                len("Konum X", t.p.y),
            ]);
            // A schedule's settings and file (docs/adr/0733 §5, §7).
            if let (Some(TableSource::Schedule { of, .. }), Some(uid)) = (&t.source, doc.model.uid(slot)) {
                let uid = uid.to_string();
                let items = vec![
                    Choice::Action {
                        label: "Çizelge ayarları…",
                        icon: "settings",
                        message: (!locked).then(|| {
                            Message::Arch(crate::arch::Event::Schedule(
                                crate::schedule_window::Event::Open(uid.clone()),
                            ))
                        }),
                    },
                    Choice::Action {
                        label: "Dosyaya yaz…",
                        icon: "tableFile",
                        message: Some(Message::Arch(crate::arch::Event::Schedule(
                            crate::schedule_window::Event::Export(uid.clone()),
                        ))),
                    },
                ];
                geo.push(Row::text("Çizelge", of.label()).editor(edit(Editor::Select {
                    text: of.label().to_owned(),
                    swatch: None,
                    icon: None,
                    items,
                })));
            }
        }
        // The block, its place, scale, turn and mirroring, through
        // `cad.entities.edit`'s properties (docs/adr/0144 §6), as the web's.
        Entity::Insert(i) => {
            let name = doc
                .model
                .block(i.block)
                .map_or_else(|| "(tanımsız)".to_owned(), |b| b.name.clone());
            let blocks = Editor::Select {
                text: name.clone(),
                swatch: None,
                icon: None,
                items: doc
                    .model
                    .blocks()
                    .iter()
                    .map(|b| Choice::Pick {
                        label: b.name.clone(),
                        swatch: None,
                        icon: None,
                        wide: false,
                        chosen: b.id == i.block,
                        enabled: true,
                        message: Message::Properties(Event::InsertBlock(slot, b.id)),
                    })
                    .collect(),
            };
            let yes_no = |m: bool| if m { "Evet" } else { "Hayır" };
            let mirror = Editor::Select {
                text: yes_no(i.mirror).to_owned(),
                swatch: None,
                icon: None,
                items: [true, false]
                    .into_iter()
                    .map(|m| Choice::Pick {
                        label: yes_no(m).to_owned(),
                        swatch: None,
                        icon: None,
                        wide: false,
                        chosen: m == i.mirror,
                        enabled: true,
                        message: Message::Properties(Event::InsertMirror(slot, m)),
                    })
                    .collect(),
            };
            // Its level (docs/adr/0733 §1): a level chosen goes to it as Kat ver writes it.
            let levels = doc.model.settings().arch.as_ref().map(|a| a.levels.clone()).unwrap_or_default();
            if !levels.is_empty() {
                let shown = i
                    .arch
                    .level
                    .as_deref()
                    .map_or_else(|| "Yok".to_owned(), |id| levels.iter().find(|l| l.id == id).map_or_else(|| id.to_owned(), |l| l.name.clone()));
                let items = levels
                    .iter()
                    .map(|l| Choice::Pick {
                        label: l.name.clone(),
                        swatch: None,
                        icon: None,
                        wide: false,
                        chosen: i.arch.level.as_deref() == Some(l.id.as_str()),
                        enabled: !locked,
                        message: Message::Properties(Event::Levels(vec![slot], l.id.clone())),
                    })
                    .collect();
                geo.push(Row::text("Kat", shown.clone()).editor(edit(Editor::Select {
                    text: shown,
                    swatch: None,
                    icon: None,
                    items,
                })));
            }
            geo.extend([
                Row::text("Blok", name).editor(edit(blocks)),
                len("Konum Y", i.p.x).editor(number(Field::InsertX(slot))),
                len("Konum X", i.p.y).editor(number(Field::InsertY(slot))),
                // One factor, or its own y factor beside the x one (docs/adr/0315 §3).
                Row::figure(
                    if i.y_scale.is_some() {
                        "X ölçeği"
                    } else {
                        "Ölçek"
                    },
                    fixed(i.scale, 4),
                )
                .editor(number(Field::InsertScale(slot))),
                Row::figure("Y ölçeği", fixed(i.y_scale.unwrap_or(i.scale), 4))
                    .editor(number(Field::InsertYScale(slot))),
                Row::figure("Dönüş", degrees(i.rotation))
                    .unit("°")
                    .editor(number(Field::InsertTurn(slot))),
                Row::text("Aynalı", yes_no(i.mirror)).editor(edit(mirror)),
            ]);
            let blocks = kentos_native_application::blocks::core_blocks(doc.model.blocks());
            if let Some([x0, y0, x1, y1]) = kentos_native_application::blocks::placed_box_in(&doc.model, &blocks, e) {
                geo.extend([len("Kutu eni (Y)", x1-x0), len("Kutu boyu (X)", y1-y0)]);
            }
            // Physical dimensions along the block's own axes; same active pieces, clip and layer gates.
            // Rotation is removed only for this read, so local dimensions and world-axis envelope stay distinct.
            if let Some(def) = doc.model.block(i.block) {
                let mut local = i.clone();
                local.p = kentos_commands::Vec2 { x:0., y:0. };
                local.rotation = 0.;
                if let Some([x0, y0, x1, y1]) = kentos_native_application::blocks::placed_box_in(&doc.model, &blocks, &Entity::Insert(local)) {
                    geo.push(Row::text("En × boy", format!("{} × {} {}",
                        f.length_bare(x1-x0), f.length_bare(y1-y0), f.length_unit_label())));
                }
                geo.push(Row::text(
                    "Bloğun birimi",
                    kentos_interaction::block_insert::unit_words(def.units),
                ));
            }
        }
        // A picture's source, place, size, turn, see-through share, clip and mirror (docs/adr/0192 §4).
        Entity::Image(im) => {
            let i = &im.image;
            let (source, linked) = match (&i.asset, &i.file) {
                (Some(a), _) => (crate::pictures::asset_words(&doc.model, a), false),
                (None, Some(path)) => {
                    let search = &doc.model.settings().xref_paths;
                    (crate::pictures::file_words(path, doc.path.as_deref(), search), true)
                }
                _ => ("—".to_owned(), false),
            };
            // Göm and Yeniden bağla… (docs/adr/0350 §1), both by `cad.images.edit`.
            let pick = |label: &str, message: Message| Choice::Pick {
                label: label.to_owned(),
                swatch: None,
                icon: None,
                wide: false,
                chosen: false,
                enabled: true,
                message,
            };
            let mut choices = Vec::new();
            if linked {
                choices.push(pick("Göm", Message::Properties(Event::EmbedImage(slot))));
            }
            choices.push(pick(
                "Yeniden bağla…",
                Message::Xrefs(crate::xrefs_window::Event::RelinkImage(slot)),
            ));
            let embed = (!locked).then(|| Editor::Select {
                text: source.clone(),
                swatch: None,
                icon: None,
                items: choices,
            });
            let yes_no = |m: bool| if m { "Evet" } else { "Hayır" };
            geo.extend([
                Row::text("Kaynak", source).editor(embed),
                len("Konum Y", i.p.x).editor(number(Field::ImageX(slot))),
                len("Konum X", i.p.y).editor(number(Field::ImageY(slot))),
                metres("Genişlik", i.width).editor(number(Field::ImageWidth(slot))),
                metres("Yükseklik", i.height).editor(number(Field::ImageHeight(slot))),
                Row::figure("Dönüş", degrees(i.rotation))
                    .unit("°")
                    .editor(number(Field::ImageTurn(slot))),
                Row::figure(
                    "Saydamlık",
                    fixed((1.0 - i.opacity.unwrap_or(1.0)) * 100.0, 0),
                )
                .unit("%")
                .editor(number(Field::ImageClear(slot))),
                Row::text("Kırpma", if i.clip.is_some() { "Var" } else { "Yok" }),
                Row::text("Aynalı", yes_no(i.mirror)),
            ]);
        }
        // Its file, size, bands, pixel, system, nodata and opacity (docs/adr/0204 §8).
        Entity::Raster(r) => geo.extend(raster::rows(doc, slot, r, locked, &f)),
        // Its files, points, bounds, density, system, index, look and opacity (docs/adr/0207 §9).
        Entity::PointCloud(c) => geo.extend(cloud::rows(doc, slot, c, locked, &f)),
        // A record has no geometry (docs/adr/0410): its attributes are its rows.
        Entity::Record(_) => {}
        // Its note, height, turn, arrowhead and mask, its corners and length (docs/adr/0146 §7).
        Entity::Leader(l) => {
            geo.extend(leader_rows(&[l], &ids, locked, &f));
            // Its style, arms, tips and content blocks (docs/adr/0326).
            geo.extend(multileader_rows(&doc.model, &[l], &ids, locked));
            geo.extend(scale_rows(&[e], &ids, locked));
            geo.push(Row::figure("Köşe sayısı", l.pts.len().to_string()));
            geo.push(metres("Uzunluk", length_of.unwrap_or(0.0)));
        }
    }

    let mut sections = vec![Section {
        id: "general".into(),
        title: "Genel".into(),
        rows: general,
    }];
    // A record has no geometry (docs/adr/0410 §4): no section for it.
    if drawn {
        sections.push(Section {
            id: "geometry".into(),
            title: "Geometri".into(),
            rows: geo,
        });
    }
    if drawn && doc.settings().workspace == Some(kentos_commands::Workspace::Cad) {
        if let Some(layer) = layers.get(&base.layer_id) {
            let look = kentos_commands::resolve_look(
                kentos_commands::OwnLook {
                    color: base.color.as_deref(),
                    line_weight: base.line_weight,
                    look: &base.look,
                },
                &layer.style,
                None,
            );
            let source = |from: kentos_commands::LookSource| {
                match from {
                    kentos_commands::LookSource::Own => "Nesne".to_owned(),
                    kentos_commands::LookSource::Layer => format!("Katman: {}", layer_path(layers, &base.layer_id)),
                    kentos_commands::LookSource::Block => "Blok".to_owned(),
                }
            };
            let pattern = if let Some(name) = base.look.line_pattern.as_deref() {
                Some(name)
            } else if base.look.line_type.is_none() {
                layer.style.line_pattern.as_deref()
            } else {
                None
            };
            let type_name = pattern.map_or_else(
                || crate::ribbon_panels::line_type_text(Some(look.line_type)).to_owned(),
                str::to_owned,
            );
            let mut appearance = vec![
                Row::text("Renk", format!("{} · {}", look.color, source(look.from[0]))),
                Row::text("Saydamlık", format!("%{} · {}", (look.transparency * 100.0).round(),
                    if look.from[4] == kentos_commands::LookSource::Own { "Nesne" } else { "Varsayılan" })),
            ];
            if e.draws_lines() {
                appearance.insert(1, Row::text("Çizgi tipi", format!("{type_name} · {}",
                    source(look.from[1]))));
                appearance.insert(2, Row::text("Çizgi kalınlığı", format!("{} · {}",
                    weight_label(look.line_weight), source(look.from[3]))));
                appearance.push(Row::text("Çizgi tipi ölçeği", format!("{} · {}",
                    plain_number(look.line_type_scale),
                    if look.from[2] == kentos_commands::LookSource::Own { "Nesne" } else { "Varsayılan" })));
            }
            sections.push(Section {
                id: "modelAppearance".into(),
                title: "Temel model görünüşü".into(),
                rows: appearance,
            });
        }
        // Where each value comes from, the standard's word on it and what a reset reaches (docs/adr/1100 §5).
        let explained = kentos_native_application::appearance_origin::explain(&doc.model, slot);
        if !explained.is_empty() {
            sections.push(Section {
                id: "valueOrigin".into(),
                title: "Bu değer nereden geliyor?".into(),
                rows: origin_rows(&explained),
            });
        }
    }
    // The kitchen run the object is part of (docs/adr/0727 §6).
    let in_run = casework::section(doc, slot, locked);
    let run_member = in_run.is_some();
    sections.extend(in_run);
    // An opening's type and its four widths (docs/adr/0772 §1, §2).
    if let Entity::Opening(o) = e {
        sections.extend(opening_types::section(doc, slot, &o.opening, &f, locked));
    }
    // A wall's components' and a slab's project materials (docs/adr/0785).
    sections.extend(materials::section(doc, slot, locked));
    // The room module the object is in (docs/adr/0775 §3).
    sections.extend(modules::section(doc, slot, locked));
    // The derived interior elevation the object is drawn in (docs/adr/0799 §5).
    sections.extend(elevations::section(doc, slot));
    // A section's placement or its mark: its state and Yeniden çiz (docs/adr/8042 §5).
    sections.extend(building_sections::section(doc, slot));
    // An insert's block attributes (docs/adr/0144 §7): the definition's tags in
    // its order, with what the insert shows (its value, else the default).
    let tags: Vec<(String, String)> = match e {
        Entity::Insert(i) => doc
            .model
            .block(i.block)
            .map(|b| {
                b.attributes
                    .iter()
                    .map(|a| {
                        let own = base.attrs.get(&a.tag).filter(|v| !v.is_empty());
                        let shown = own.or(a.value.as_ref()).cloned().unwrap_or_default();
                        (a.tag.clone(), shown)
                    })
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    // A dynamic block's parameters (docs/adr/0319 §3).
    if let Entity::Insert(i) = e {
        sections.extend(params::sections(doc, slot, i, locked));
    }
    if !tags.is_empty() {
        sections.push(Section {
            id: "blockAttrs".into(),
            title: "Blok öznitelikleri".into(),
            rows: tags
                .iter()
                .map(|(tag, value)| Row {
                    label: Cow::Owned(tag.clone()),
                    value: value.clone(),
                    numeric: looks_numeric(value),
                    unit: None,
                    editor: edit(Editor::Text(Field::Attribute(slot, tag.clone()))),
                    note: false,
                    warn: false,
                    thumb: None,
                })
                .collect(),
        });
    }
    // The layer's fields first, each by its kind (docs/adr/0199 §5); the keys no field names after them, as text.
    let fields = layers
        .get(&base.layer_id)
        .map(|n| n.fields.clone())
        .unwrap_or_default();
    if !fields.is_empty() {
        let mut rows = form_rows(doc.model.settings(), &base.layer_id);
        let labels = crate::features::lookup_labels(&doc.model, &base.layer_id, &fields);
        let subtypes = layers.get(&base.layer_id).and_then(|n| n.subtypes.clone());
        let groups = layers
            .get(&base.layer_id)
            .map(|n| n.contingents.clone())
            .unwrap_or_default();
        rows.extend(field_rows(
            slot,
            base,
            &fields,
            subtypes.as_ref(),
            &groups,
            locked,
            &labels,
        ));
        sections.push(Section {
            id: "fields".into(),
            title: "Alanlar".into(),
            rows,
        });
    }
    let others: Vec<(&String, &String)> = base
        .attrs
        .iter()
        .filter(|(key, _)| !tags.iter().any(|(tag, _)| tag == *key))
        .filter(|(key, _)| !fields.iter().any(|f| f.name == **key))
        // A run's data is its run's: its section edits it (docs/adr/0727 §5).
        .filter(|(key, _)| !(run_member && matches!(key.as_str(), "Modül" | "Tür" | "Açılma")))
        .collect();
    if !others.is_empty() {
        sections.push(Section {
            id: "attrs".into(),
            title: "Öznitelik bilgileri".into(),
            rows: others
                .into_iter()
                .map(|(key, value)| Row {
                    label: Cow::Owned(key.clone()),
                    value: value.clone(),
                    numeric: looks_numeric(value),
                    unit: None,
                    editor: edit(Editor::Text(Field::Attribute(slot, key.clone()))),
                    note: false,
                    warn: false,
                    thumb: None,
                })
                .collect(),
        });
    }
    building_first(e, &mut sections);
    sections
}
