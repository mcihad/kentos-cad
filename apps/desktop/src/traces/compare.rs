//! What a step expects against what the app shows, by the web runner's
//! rules: clicked points and the view's centre within `clickTolerance`, typed
//! edges exactly, the scale within a relative 1e-9.

use super::format::{Expect, Newest, Trace};
use super::player::{Observation, Seen};

/// Differences between what a step expects and what the app shows, by the
/// web runner's rules; empty when it matches.
pub fn compare(expect: &Expect, got: &Observation, trace: &Trace) -> Vec<String> {
    let mut bad = Vec::new();
    let mut check = |name: &str, same: bool, have: String, want: String| {
        if !same {
            bad.push(format!("{name}: {have}, beklenen {want}"));
        }
    };
    if let Some(want) = &expect.tool {
        check("tool", &got.tool == want, got.tool.clone(), want.clone());
    }
    if let Some(want) = expect.points {
        check(
            "points",
            got.points == want,
            got.points.to_string(),
            want.to_string(),
        );
    }
    if let Some(want) = &expect.prompt {
        check(
            "prompt",
            &got.prompt == want,
            format!("{:?}", got.prompt),
            format!("{want:?}"),
        );
    }
    if let Some(want) = &expect.logged {
        // Each whole and in this order among the step's messages; others may come between.
        let mut said = got.messages.iter();
        check(
            "logged",
            want.iter().all(|w| said.any(|m| m == w)),
            format!("{:?}", got.messages),
            format!("{want:?}"),
        );
    }
    if let Some(want) = &expect.options {
        check(
            "options",
            &got.options == want,
            format!("{:?}", got.options),
            format!("{want:?}"),
        );
    }
    if let Some(want) = &expect.dynamic_input {
        check(
            "dynamicInput",
            &got.dynamic_input == want,
            format!("{:?}", got.dynamic_input),
            format!("{want:?}"),
        );
    }
    if let Some(want) = &expect.locks {
        check(
            "locks",
            &got.locks == want,
            format!("{:?}", got.locks),
            format!("{want:?}"),
        );
    }
    if let Some(want) = &expect.command_line {
        check(
            "commandLine",
            &got.command_line == want,
            format!("{:?}", got.command_line),
            format!("{want:?}"),
        );
    }
    if let Some(want) = expect.entities {
        check(
            "entities",
            got.entities == want,
            got.entities.to_string(),
            want.to_string(),
        );
    }
    if let Some(want) = expect.can_undo {
        check(
            "canUndo",
            got.can_undo == want,
            got.can_undo.to_string(),
            want.to_string(),
        );
    }
    if let Some(want) = expect.can_redo {
        check(
            "canRedo",
            got.can_redo == want,
            got.can_redo.to_string(),
            want.to_string(),
        );
    }
    if let Some(want) = expect.dirty {
        check(
            "dirty",
            got.dirty == want,
            got.dirty.to_string(),
            want.to_string(),
        );
    }
    if let Some(want) = &expect.log {
        check(
            "log",
            got.log.as_ref() == Some(want),
            format!("{:?}", got.log),
            want.clone(),
        );
    }
    if let Some(want) = expect.metres_per_pixel {
        check(
            "metresPerPixel",
            (got.metres_per_pixel - want).abs() <= want * 1e-9,
            got.metres_per_pixel.to_string(),
            want.to_string(),
        );
    }
    if let Some([wx, wy]) = expect.view_center {
        // Where Kaydır and the zooms put the view: from clicks, within the click tolerance.
        let [ox, oy] = trace.view.center;
        let have = [got.view_center[0] - ox, got.view_center[1] - oy];
        check(
            "viewCenter",
            (have[0] - wx).hypot(have[1] - wy) <= trace.click_tolerance,
            format!("{have:?}"),
            format!("{:?} (±{} m)", [wx, wy], trace.click_tolerance),
        );
    }
    // Object tracking (docs/adr/0085): points within the click tolerance, angles exact.
    let rel = |p: [f64; 2]| [p[0] - trace.view.center[0], p[1] - trace.view.center[1]];
    let near = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]) <= trace.click_tolerance;
    if let Some(want) = &expect.track_points {
        let have: Vec<[f64; 2]> = got.track_points.iter().map(|&p| rel(p)).collect();
        check(
            "trackPoints",
            have.len() == want.len() && have.iter().zip(want).all(|(a, b)| near(*a, *b)),
            format!("{have:?}"),
            format!("{want:?}"),
        );
    }
    if let Some(want) = &expect.track {
        let have = got.track.as_ref().map(|t| {
            (
                rel(t.point),
                t.lines
                    .iter()
                    .map(|(o, a)| (rel(*o), *a))
                    .collect::<Vec<_>>(),
            )
        });
        let same = match (want, &have) {
            (None, None) => true,
            (Some(w), Some((point, lines))) => {
                near(*point, w.point)
                    && lines.len() == w.lines.len()
                    && lines
                        .iter()
                        .zip(&w.lines)
                        .all(|((o, a), l)| near(*o, l.origin) && *a == l.angle)
            }
            _ => false,
        };
        let want_text = want.as_ref().map(|w| {
            let lines: Vec<String> = w
                .lines
                .iter()
                .map(|l| format!("{:?} {}°", l.origin, l.angle))
                .collect();
            format!("{:?} [{}]", w.point, lines.join(", "))
        });
        check("track", same, format!("{have:?}"), format!("{want_text:?}"));
    }
    if let Some(want) = &expect.selected {
        check(
            "selected",
            &got.selected == want,
            format!("{:?}", got.selected),
            format!("{want:?}"),
        );
    }
    if let Some(want) = &expect.hover {
        check(
            "hover",
            &got.hover == want,
            format!("{:?}", got.hover),
            format!("{want:?}"),
        );
    }
    if let Some(want) = &expect.cycle {
        check(
            "cycle",
            &got.cycle == want,
            format!("{:?}", got.cycle),
            format!("{want:?}"),
        );
    }
    if let Some(want) = &expect.dialog {
        check(
            "dialog",
            &got.dialog == want,
            format!("{:?}", got.dialog),
            format!("{want:?}"),
        );
    }
    // The bottom panel's tab, Arama's count and rows, and the place the data
    // search marked (docs/adr/0178): the first two exact, the place within the click tolerance.
    if let Some(want) = &expect.panel {
        check(
            "panel",
            &got.panel == want,
            format!("{:?}", got.panel),
            format!("{want:?}"),
        );
    }
    if let Some(want) = &expect.search {
        let have = got.search.clone();
        check(
            "search",
            have.as_ref() == Some(&(want.count.clone(), want.rows.clone())),
            format!("{have:?}"),
            format!("{:?}", (&want.count, &want.rows)),
        );
    }
    if let Some(want) = &expect.feature_table {
        let have = got.feature_table.clone();
        let same = have.as_ref().is_some_and(|(count, columns, rows)| {
            *count == want.count
                && *rows == want.rows
                && want.columns.as_ref().is_none_or(|c| c == columns)
        });
        check(
            "featureTable",
            same,
            format!("{have:?}"),
            format!("{:?}", (&want.count, &want.columns, &want.rows)),
        );
    }
    if let Some(want) = &expect.mark {
        let have = got
            .mark
            .map(|p| [p[0] - trace.view.center[0], p[1] - trace.view.center[1]]);
        let same = match (have, want) {
            (None, None) => true,
            (Some(h), Some(w)) => (h[0] - w[0]).hypot(h[1] - w[1]) <= trace.click_tolerance,
            _ => false,
        };
        check(
            "mark",
            same,
            format!("{have:?}"),
            format!("{want:?} (±{} m)", trace.click_tolerance),
        );
    }
    // Genel bakış's extent, east and north of the set-up's centre (docs/adr/0181).
    if let Some(want) = &expect.overview {
        let c = trace.view.center;
        let have = got
            .overview
            .map(|e| e.map(|[x0, y0, x1, y1]| [x0 - c[0], y0 - c[1], x1 - c[0], y1 - c[1]]));
        let same = match (have, want) {
            (None, None) => true,
            (Some(Some(h)), Some(w)) => h.iter().zip(w.extent).all(|(a, b)| (a - b).abs() <= 1e-6),
            _ => false,
        };
        check(
            "overview",
            same,
            format!("{have:?}"),
            format!("{:?}", want.as_ref().map(|w| w.extent)),
        );
    }
    // Büyüteç's zoom and side exactly, its centre within the click tolerance.
    if let Some(want) = &expect.magnifier {
        let c = trace.view.center;
        let have = got
            .magnifier
            .map(|(zoom, side, at)| (zoom, side, at.map(|p| [p[0] - c[0], p[1] - c[1]])));
        let same = match (have, want) {
            (None, None) => true,
            (Some((zoom, side, at)), Some(w)) => {
                zoom == w.zoom
                    && w.side.as_deref().is_none_or(|s| s == side)
                    && w.center.is_none_or(|wc| {
                        at.is_some_and(|h| {
                            (h[0] - wc[0]).hypot(h[1] - wc[1]) <= trace.click_tolerance
                        })
                    })
            }
            _ => false,
        };
        check(
            "magnifier",
            same,
            format!("{have:?}"),
            format!("{want:?} (±{} m)", trace.click_tolerance),
        );
    }
    // The active layer and the current colour and weight (docs/adr/0176 §3), exact.
    if let Some(want) = &expect.active_layer {
        check(
            "activeLayer",
            &got.active_layer == want,
            format!("{:?}", got.active_layer),
            format!("{want:?}"),
        );
    }
    // The layers and groups hidden or locked of their own (docs/adr/0177 §1), exact.
    if let Some(want) = &expect.hidden_layers {
        check(
            "hiddenLayers",
            &got.hidden_layers == want,
            format!("{:?}", got.hidden_layers),
            format!("{want:?}"),
        );
    }
    if let Some(want) = &expect.locked_layers {
        check(
            "lockedLayers",
            &got.locked_layers == want,
            format!("{:?}", got.locked_layers),
            format!("{want:?}"),
        );
    }
    // Every layer and group (docs/adr/0177 §5), exact.
    if let Some(want) = &expect.layers {
        check(
            "layers",
            &got.layers == want,
            format!("{:?}", got.layers),
            format!("{want:?}"),
        );
    }
    if let Some(want) = &expect.current_color {
        check(
            "currentColor",
            &got.current_color == want,
            format!("{:?}", got.current_color),
            format!("{want:?}"),
        );
    }
    if let Some(want) = &expect.current_weight {
        check(
            "currentWeight",
            &got.current_weight == want,
            format!("{:?}", got.current_weight),
            format!("{want:?}"),
        );
    }
    if let Some(want) = &expect.snap {
        check(
            "snap",
            &got.snap == want,
            format!("{:?}", got.snap),
            format!("{want:?}"),
        );
    }
    if let Some(want) = &expect.ids {
        check(
            "ids",
            &got.ids == want,
            format!("{:?}", got.ids),
            format!("{want:?}"),
        );
    }
    if let Some(want) = &expect.newest {
        bad.extend(compare_shape("newest", want, got.newest.as_ref(), trace));
    }
    // Objects by their ids (docs/adr/0037): each compared as `newest` is.
    for want in expect.objects.iter().flatten() {
        let name = format!("objects[{}]", want.id.unwrap_or_default());
        let seen = want.id.and_then(|id| got.objects.get(&id));
        bad.extend(compare_shape(&name, want, seen, trace));
    }
    bad
}

/// An object's expected shape against what it is: `name` (`newest`,
/// `objects[2]`) begins each difference.
/// A table's differences from what a step expects (the web's `compareTable`).
fn compare_table(
    name: &str,
    want: &super::format::TableExpect,
    seen: Option<&super::player::TableSeen>,
) -> Vec<String> {
    let Some(seen) = seen else {
        return vec![format!("{name}.table: yok, beklenen bir tablo")];
    };
    let mut bad = Vec::new();
    let near = |a: &[f64], b: &[f64], tolerance: f64| {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tolerance)
    };
    if let Some(cells) = &want.cells
        && seen.cells != *cells
    {
        bad.push(format!(
            "{name}.table.cells: {:?}, beklenen {cells:?}",
            seen.cells
        ));
    }
    if let Some(rows) = &want.rows
        && !near(&seen.rows, rows, 1e-6)
    {
        bad.push(format!(
            "{name}.table.rows: {:?}, beklenen {rows:?}",
            seen.rows
        ));
    }
    if let Some(columns) = &want.columns
        && !near(&seen.columns, columns, 1e-6)
    {
        bad.push(format!(
            "{name}.table.columns: {:?}, beklenen {columns:?}",
            seen.columns
        ));
    }
    if let Some(merges) = &want.merges
        && seen.merges != *merges
    {
        bad.push(format!(
            "{name}.table.merges: {:?}, beklenen {merges:?}",
            seen.merges
        ));
    }
    if let Some(aligns) = &want.aligns
        && seen.aligns != *aligns
    {
        bad.push(format!(
            "{name}.table.aligns: {:?}, beklenen {aligns:?}",
            seen.aligns
        ));
    }
    if let Some(header) = want.header
        && seen.header != header
    {
        bad.push(format!(
            "{name}.table.header: {}, beklenen {header}",
            seen.header
        ));
    }
    if let Some(grid) = &want.grid
        && seen.grid != *grid
    {
        bad.push(format!(
            "{name}.table.grid: {:?}, beklenen {grid:?}",
            seen.grid
        ));
    }
    if let Some(frame) = want.frame {
        let same = match (frame, seen.frame) {
            (None, None) => true,
            (Some(a), Some(b)) => (a - b).abs() <= 1e-9,
            _ => false,
        };
        if !same {
            bad.push(format!(
                "{name}.table.frame: {:?}, beklenen {frame:?}",
                seen.frame
            ));
        }
    }
    if let Some(source) = &want.source
        && seen.source != *source
    {
        bad.push(format!(
            "{name}.table.source: {:?}, beklenen {source:?}",
            seen.source
        ));
    }
    bad
}

fn compare_shape(name: &str, want: &Newest, seen: Option<&Seen>, trace: &Trace) -> Vec<String> {
    let mut bad = Vec::new();
    let Some(seen) = seen else {
        return vec![format!("{name}: yok, beklenen {}", want.kind)];
    };
    let (kind, pts, bulges) = (&seen.kind, &seen.pts, &seen.bulges);
    if *kind != want.kind {
        return vec![format!("{name}: {kind}, beklenen {}", want.kind)];
    }
    let [ox, oy] = trace.view.center;
    // A circle's or an arc's centre and radius: from clicks, within the click tolerance.
    if let Some([wx, wy]) = want.center {
        let have = seen.center.map(|[x, y]| [x - ox, y - oy]);
        if !have.is_some_and(|[x, y]| (x - wx).hypot(y - wy) <= trace.click_tolerance) {
            bad.push(format!(
                "{name}.center: {have:?}, beklenen {:?} (±{} m)",
                [wx, wy],
                trace.click_tolerance
            ));
        }
    }
    if let Some(r) = want.radius
        && !seen
            .radius
            .is_some_and(|have| (have - r).abs() <= trace.click_tolerance)
    {
        bad.push(format!(
            "{name}.radius: {:?}, beklenen {r} (±{} m)",
            seen.radius, trace.click_tolerance
        ));
    }
    if let Some(points) = &want.points {
        // Clicked points come from screen pixels: within the trace's tolerance.
        let near = pts.len() == points.len()
            && pts.iter().zip(points).all(|([x, y], [wx, wy])| {
                (x - ox - wx).hypot(y - oy - wy) <= trace.click_tolerance
            });
        if !near {
            let relative: Vec<[f64; 2]> = pts.iter().map(|[x, y]| [x - ox, y - oy]).collect();
            bad.push(format!(
                "{name}.points: {relative:?}, beklenen {points:?} (±{} m)",
                trace.click_tolerance
            ));
        }
    }
    // A text's content is exact (docs/adr/0144 §7: an exploded attribute's value).
    if let Some(text) = &want.text
        && seen.text.as_ref() != Some(text)
    {
        bad.push(format!("{name}.text: {:?}, beklenen {text:?}", seen.text));
    }
    // A text's alignment, width factor and mask (docs/adr/0145), exact.
    if let Some(align) = &want.align
        && seen.align != *align
    {
        bad.push(format!(
            "{name}.align: {:?}, beklenen {align:?}",
            seen.align
        ));
    }
    if let Some(factor) = want.width_factor
        && seen.width_factor != Some(factor)
    {
        bad.push(format!(
            "{name}.widthFactor: {:?}, beklenen {factor}",
            seen.width_factor
        ));
    }
    if let Some(mask) = want.mask
        && seen.mask != Some(mask)
    {
        bad.push(format!("{name}.mask: {:?}, beklenen {mask}", seen.mask));
    }
    // A text's curve's vertex count (docs/adr/0196), exact.
    if let Some(curve) = want.curve
        && seen.curve != curve
    {
        bad.push(format!(
            "{name}.curve: {:?}, beklenen {curve:?}",
            seen.curve
        ));
    }
    // A linked text's object and scale (docs/adr/0175 §4), exact.
    if let Some(of) = want.label_of
        && seen.label_of != of
    {
        bad.push(format!(
            "{name}.labelOf: {:?}, beklenen {of:?}",
            seen.label_of
        ));
    }
    if let Some(scale) = want.label_scale
        && seen.label_scale != Some(scale)
    {
        bad.push(format!(
            "{name}.labelScale: {:?}, beklenen {scale}",
            seen.label_scale
        ));
    }
    // A multi-line text's box (from clicks: within the click tolerance), spacing and runs (docs/adr/0182).
    if let Some(width) = want.box_width {
        let near = match (width, seen.box_width) {
            (None, None) => true,
            (Some(w), Some(have)) => (have - w).abs() <= trace.click_tolerance,
            _ => false,
        };
        if !near {
            bad.push(format!(
                "{name}.boxWidth: {:?}, beklenen {width:?} (±{} m)",
                seen.box_width, trace.click_tolerance
            ));
        }
    }
    if let Some(spacing) = want.line_spacing
        && seen.line_spacing != spacing
    {
        bad.push(format!(
            "{name}.lineSpacing: {:?}, beklenen {spacing:?}",
            seen.line_spacing
        ));
    }
    if let Some(runs) = &want.runs
        && seen.runs != *runs
    {
        bad.push(format!("{name}.runs: {:?}, beklenen {runs:?}", seen.runs));
    }
    // A leader's arrowhead (docs/adr/0146), exact.
    if let Some(arrow) = &want.arrow
        && seen.arrow != *arrow
    {
        bad.push(format!(
            "{name}.arrow: {:?}, beklenen {arrow:?}",
            seen.arrow
        ));
    }
    // A dimension's direction (docs/adr/0147), within 1e-9.
    if let Some(angle) = want.angle
        && !seen.angle.is_some_and(|a| (a - angle).abs() <= 1e-9)
    {
        bad.push(format!(
            "{name}.angle: {:?}, beklenen {angle}",
            seen.angle
        ));
    }
    // A text's height (docs/adr/0175), a dimension's value's (docs/adr/0183), within 1e-9.
    if let Some(height) = want.height
        && !seen.height.is_some_and(|h| (h - height).abs() <= 1e-9)
    {
        bad.push(format!(
            "{name}.height: {:?}, beklenen {height}",
            seen.height
        ));
    }
    // A table's cells and look (docs/adr/0184).
    if let Some(table) = &want.table {
        bad.extend(compare_table(name, table, seen.table.as_ref()));
    }
    // A text's style and face, a dimension's style and look (docs/adr/0183), exact.
    if let Some(face) = &want.face
        && seen.face.as_ref() != Some(face)
    {
        bad.push(format!("{name}.face: {:?}, beklenen {face:?}", seen.face));
    }
    if let Some(look) = &want.look
        && seen.look.as_ref() != Some(look)
    {
        bad.push(format!("{name}.look: {:?}, beklenen {look:?}", seen.look));
    }
    if let Some(rotation) = want.rotation
        && seen.rotation != Some(rotation)
    {
        bad.push(format!(
            "{name}.rotation: {:?}, beklenen {rotation}",
            seen.rotation
        ));
    }
    // A survey point's name, attributes and elevation (docs/adr/0152), exact.
    if let Some(label) = &want.label
        && seen.label != *label
    {
        bad.push(format!(
            "{name}.label: {:?}, beklenen {label:?}",
            seen.label
        ));
    }
    if let Some(attrs) = &want.attrs
        && seen.attrs != *attrs
    {
        bad.push(format!(
            "{name}.attrs: {:?}, beklenen {attrs:?}",
            seen.attrs
        ));
    }
    if let Some(z) = want.z
        && seen.z != z
    {
        bad.push(format!("{name}.z: {:?}, beklenen {z:?}", seen.z));
    }
    // An object template's symbol, colour, weight and layer (docs/adr/0176 §3), exact.
    if let Some(symbol) = &want.symbol
        && seen.symbol != *symbol
    {
        bad.push(format!(
            "{name}.symbol: {:?}, beklenen {symbol:?}",
            seen.symbol
        ));
    }
    if let Some(color) = &want.color
        && seen.color != *color
    {
        bad.push(format!(
            "{name}.color: {:?}, beklenen {color:?}",
            seen.color
        ));
    }
    if let Some(weight) = want.line_weight
        && seen.line_weight != weight
    {
        bad.push(format!(
            "{name}.lineWeight: {:?}, beklenen {weight:?}",
            seen.line_weight
        ));
    }
    if let Some(layer) = &want.layer
        && seen.layer != *layer
    {
        bad.push(format!(
            "{name}.layer: {:?}, beklenen {layer:?}",
            seen.layer
        ));
    }
    // A hatch's pattern and how many objects it follows (docs/adr/0186), exact.
    if let Some(pattern) = &want.pattern
        && seen.pattern.as_ref() != Some(pattern)
    {
        bad.push(format!(
            "{name}.pattern: {:?}, beklenen {pattern:?}",
            seen.pattern
        ));
    }
    if let Some(assoc) = want.assoc
        && seen.assoc != Some(assoc)
    {
        bad.push(format!("{name}.assoc: {:?}, beklenen {assoc}", seen.assoc));
    }
    // An area's hole count (docs/adr/0173 §5), exact.
    if let Some(holes) = want.holes
        && seen.holes != Some(holes)
    {
        bad.push(format!("{name}.holes: {:?}, beklenen {holes}", seen.holes));
    }
    // A shared arc and an elevation along an edge (docs/adr/0160) come from
    // the core's arithmetic: within 1e-9.
    if let Some(want_bulges) = &want.bulges {
        let at = |i: usize| bulges.get(i).copied().unwrap_or(0.0);
        let near = want_bulges
            .iter()
            .enumerate()
            .all(|(i, b)| (at(i) - b).abs() <= 1e-9)
            && bulges.iter().skip(want_bulges.len()).all(|b| *b == 0.0);
        if !near {
            bad.push(format!(
                "{name}.bulges: {bulges:?}, beklenen {want_bulges:?} (±1e-9)"
            ));
        }
    }
    if let Some(want_zs) = &want.zs {
        let near = seen.zs.len() == want_zs.len()
            && seen
                .zs
                .iter()
                .zip(want_zs)
                .all(|(have, want)| match (have, want) {
                    (Some(h), Some(w)) => (h - w).abs() <= 1e-9,
                    (None, None) => true,
                    _ => false,
                });
        if !near {
            bad.push(format!(
                "{name}.zs: {:?}, beklenen {want_zs:?} (±1e-9)",
                seen.zs
            ));
        }
    }
    if let Some(arcs) = want.arcs {
        let have = bulges.iter().filter(|b| **b != 0.0).count();
        if have != arcs {
            bad.push(format!("{name}.arcs: {have}, beklenen {arcs}"));
        }
    }
    if let Some(edges) = &want.edges {
        // Typed values are exact: consecutive corner differences, not rounded.
        let have: Vec<[f64; 2]> = pts
            .windows(2)
            .map(|w| [w[1][0] - w[0][0], w[1][1] - w[0][1]])
            .collect();
        if have != *edges {
            bad.push(format!("{name}.edges: {have:?}, beklenen {edges:?}"));
        }
    }
    bad
}
