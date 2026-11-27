//! The ground a section draws (docs/adr/8174; EK-4 4.30; Planlı Alanlar İmar Yönetmeliği 4, 57/2-a: "tabii ve tesviye
//! zemin" in every section). Where the site names the natural and the graded ground's terrain surfaces and ±0.00's
//! elevation, each surface's section along the line (exact where it crosses a triangle's edge, the road's own
//! `Surface::section`), its heights relative to ±0.00: the natural ground light and dashed, the graded ground heavy.
//! Without them the graded ground lies flat at ±0.00, as before. Neither runs through what is cut.

use kentos_road::surface::Surface;

use super::*;

/// The natural ground's layer (EK-2 Çevre).
const NATURAL: &str = "MM-CVRE-TABII";
/// The graded ground's: the flat line's, as before.
const GRADED: &str = "MM-CVRE-TESVIYE";

/// The site's grounds, made from their surfaces now, and ±0.00's elevation.
pub(super) struct Grounds {
    natural: Option<Surface>,
    graded: Option<Surface>,
    datum: f64,
}

/// The site's grounds: none when it names neither surface, gives no ±0.00 elevation or neither surface is made.
pub(super) fn grounds(doc: &Document) -> Option<Grounds> {
    let site = doc.settings().arch.as_ref()?.site.as_ref()?;
    let datum = site.datum?;
    let made = |id: Option<&str>| {
        let def = crate::road_terrains::surfaces(doc).iter().find(|d| Some(d.id.as_str()) == id)?;
        crate::road_terrains::made_of(doc, def).1
    };
    let g = Grounds {
        natural: made(site.natural_ground.as_deref()),
        graded: made(site.graded_ground.as_deref()),
        datum,
    };
    (g.natural.is_some() || g.graded.is_some()).then_some(g)
}

/// Whether the model holds the natural ground; a section lists it as left out otherwise (§3).
pub(super) fn natural_known(doc: &Document) -> bool {
    grounds(doc).is_some_and(|g| g.natural.is_some())
}

/// The ground across `span` (along the line, the section's `u`), not through what is cut.
pub(super) fn draw(doc: &Document, frame: &Frame, cut: &[sec::Cut], span: (f64, f64), out: &mut Out) {
    let g = grounds(doc);
    if let Some(s) = g.as_ref().and_then(|g| g.natural.as_ref().map(|s| (s, g.datum))) {
        drape(s.0, frame, cut, span, s.1, (NATURAL, LIGHT, Some("dashed")), out);
    }
    match g.as_ref().and_then(|g| g.graded.as_ref().map(|s| (s, g.datum))) {
        Some((s, datum)) => drape(s, frame, cut, span, datum, (GRADED, HEAVY, None), out),
        None => {
            for (a, b) in uncut(cut, span, 0.0) {
                out.line(GRADED, P::new(a, 0.0), P::new(b, 0.0), HEAVY);
            }
        }
    }
}

/// Whether `at` lies inside `ring` (even-odd).
fn inside(ring: &[P], at: P) -> bool {
    let mut odd = false;
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        if (a.y > at.y) != (b.y > at.y) && at.x < a.x + (at.y - a.y) * (b.x - a.x) / (b.y - a.y) {
            odd = !odd;
        }
    }
    odd
}

/// The ground of `s` along the line over `span`: its section's places relative to ±0.00 (`datum`), broken where the
/// line leaves the surface and where it passes through what is cut, each run a polyline on its layer.
fn drape(
    s: &Surface,
    frame: &Frame,
    cut: &[sec::Cut],
    span: (f64, f64),
    datum: f64,
    (layer, weight, line_type): (&str, f64, Option<&str>),
    out: &mut Out,
) {
    let (a, b) = (frame.at(span.0, 0.0), frame.at(span.1, 0.0));
    let places = s.section(&[[a.x, a.y], [b.x, b.y]]);
    // The places along the line (u, height over ±0.00), a stretch off the surface breaking it.
    let mut runs: Vec<Vec<P>> = Vec::new();
    let mut run: Vec<P> = Vec::new();
    fn end(run: &mut Vec<P>, runs: &mut Vec<Vec<P>>) {
        if run.len() > 1 {
            runs.push(std::mem::take(run));
        }
        run.clear();
    }
    // Every cut ring's corners break a stretch, so that a piece lies wholly in or out of what is cut.
    let breaks: Vec<f64> = cut.iter().flat_map(|c| c.ring.iter().map(|q| q.x)).collect();
    for w in places.windows(2) {
        let (Some(z0), Some(z1)) = (w[0].z, w[1].z) else {
            end(&mut run, &mut runs);
            continue;
        };
        if w[1].gap {
            end(&mut run, &mut runs);
            continue;
        }
        let (u0, u1) = (span.0 + w[0].station, span.0 + w[1].station);
        let at = |u: f64| {
            let t = if u1 > u0 { (u - u0) / (u1 - u0) } else { 0.0 };
            P::new(u, z0 - datum + (z1 - z0) * t)
        };
        let mut cuts: Vec<f64> = breaks.iter().copied().filter(|u| *u > u0 && *u < u1).collect();
        cuts.sort_by(f64::total_cmp);
        let mut from = u0;
        for to in cuts.into_iter().chain(std::iter::once(u1)) {
            let middle = at((from + to) / 2.0);
            if cut.iter().any(|c| inside(&c.ring, middle)) {
                end(&mut run, &mut runs);
            } else {
                if run.is_empty() {
                    run.push(at(from));
                }
                run.push(at(to));
            }
            from = to;
        }
    }
    end(&mut run, &mut runs);
    for r in runs {
        let pts: Vec<Value> = r.iter().map(|q| json!({"x": q.x, "y": q.y})).collect();
        let mut v = json!({"kind": "polyline", "pts": pts, "closed": false, "lineWeight": weight});
        if let Some(t) = line_type {
            v["lineType"] = json!(t);
        }
        out.put(layer, v);
    }
}
