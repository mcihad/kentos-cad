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
    if let Some(want) = &expect.dialog {
        check(
            "dialog",
            &got.dialog == want,
            format!("{:?}", got.dialog),
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
    // A leader's arrowhead (docs/adr/0146), exact.
    if let Some(arrow) = &want.arrow
        && seen.arrow != *arrow
    {
        bad.push(format!(
            "{name}.arrow: {:?}, beklenen {arrow:?}",
            seen.arrow
        ));
    }
    if let Some(rotation) = want.rotation
        && seen.rotation != Some(rotation)
    {
        bad.push(format!(
            "{name}.rotation: {:?}, beklenen {rotation}",
            seen.rotation
        ));
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
