//! Mimari denetim's door group (docs/adr/0772 §5): three kinds of finding, kept apart. A physical clash
//! (`arch_door_swing_clash`): a door's zone as it opens (a swinging leaf's sweep, a revolving door's drum, a folding
//! door's gathered panels, a one-leaf sliding door's travel on its track; docs/adr/0787 §1) runs into another wall, a
//! column, a fixture or another door's zone; its fix turns the door to the other face or hinge side when that one
//! clears everything (`cad.openings.edit`'s Çevir). Passage (`arch_door_front_blocked`): its use zone (`build.useZone`) is taken by one of those. A project
//! rule (`arch_door_clear_narrow`): its net clear width is under the minimum the audit is given. Walls and arcs are
//! sampled at their vertices and every [`ARC_STEP`] radians; a touch along an edge is no clash (each zone shrunk by
//! [`SLACK`] about its middle).

use kentos_commands::{
    ArchAuditCheck, ArchAuditFinding, ArchAuditNature, Entity, OpeningForm, OpeningOperation,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2 as V;
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::geom::bulge::bulge_path_edges;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::geom::opening;
use kentos_geometry_core::geom::wall as wall_geom;

/// How far a zone is shrunk before it is tested, metres: a touch is no clash.
pub const SLACK: f64 = 0.001;
/// The step arcs are sampled at, radians.
const ARC_STEP: f64 = 0.1;

/// An obstacle: its object, what it is, its rings (an outer one and holes).
struct Obstacle {
    slot: Slot,
    level: Option<String>,
    what: &'static str,
    outer: Vec<V>,
    holes: Vec<Vec<V>>,
}

fn points_of(edges: &[Edge]) -> Vec<V> {
    let mut out = Vec::new();
    for e in edges {
        match *e {
            Edge::Seg { a, .. } => out.push(a),
            Edge::Arc { c, r, a0, sweep } => {
                let n = ((sweep.abs() / ARC_STEP).ceil() as usize).max(1);
                for k in 0..n {
                    let t = a0 + sweep * k as f64 / n as f64;
                    out.push(V::new(c.x + r * t.cos(), c.y + r * t.sin()));
                }
            }
        }
    }
    out
}

fn wall_rings(w: &kentos_commands::WallFields) -> Option<(Vec<V>, Vec<Vec<V>>)> {
    let spec = kentos_formats::walls::spec(w);
    let pts: Vec<V> = w.pts.iter().map(|p| V::new(p.x, p.y)).collect();
    let lines = wall_geom::lines(&pts, w.bulges.as_deref(), &spec).ok()?;
    let (left, right) = (lines.first()?, lines.last()?);
    let b = |l: &wall_geom::Line| (!l.bulges.is_empty()).then(|| l.bulges.clone());
    let l = points_of(&bulge_path_edges(
        &left.pts,
        b(left).as_deref(),
        spec.closed,
    ));
    let r = points_of(&bulge_path_edges(
        &right.pts,
        b(right).as_deref(),
        spec.closed,
    ));
    if spec.closed {
        // The body between the two face rings: the larger the outer.
        let (outer, inner) = if area(&l).abs() >= area(&r).abs() {
            (l, r)
        } else {
            (r, l)
        };
        return Some((outer, vec![inner]));
    }
    let mut ring = left.pts.clone();
    if let Some(last) = right.pts.last() {
        ring.push(*last);
    }
    ring.extend(right.pts.iter().rev().skip(1).copied());
    if ring.len() < 3 {
        return None;
    }
    // Arcs between the faces' points: sample the faces as chords' points when they bend.
    if !left.bulges.iter().all(|x| *x == 0.0) || !right.bulges.iter().all(|x| *x == 0.0) {
        let mut ring = l;
        ring.extend(r.into_iter().rev());
        return Some((ring, Vec::new()));
    }
    Some((ring, Vec::new()))
}

fn area(ring: &[V]) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum::<f64>()
        / 2.0
}

fn inside(ring: &[V], p: V) -> bool {
    let mut odd = false;
    let n = ring.len();
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + n - 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            odd = !odd;
        }
    }
    odd
}

fn cross(o: V, a: V, b: V) -> f64 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

fn crosses(a: V, b: V, c: V, d: V) -> bool {
    let (d1, d2, d3, d4) = (
        cross(c, d, a),
        cross(c, d, b),
        cross(a, b, c),
        cross(a, b, d),
    );
    ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
}

fn edges_cross(a: &[V], b: &[V]) -> bool {
    let (n, m) = (a.len(), b.len());
    (0..n).any(|i| (0..m).any(|j| crosses(a[i], a[(i + 1) % n], b[j], b[(j + 1) % m])))
}

/// Whether the zone `z` and the obstacle overlap: an edge of one crosses the other's, or a point of one lies inside the
/// other (inside its outer ring, outside its holes).
fn overlaps(z: &[V], o: &Obstacle) -> bool {
    if !boxes_meet(z, &o.outer) {
        return false;
    }
    let in_body = |p: V| inside(&o.outer, p) && !o.holes.iter().any(|h| inside(h, p));
    edges_cross(z, &o.outer)
        || o.holes.iter().any(|h| edges_cross(z, h))
        || z.iter().any(|p| in_body(*p))
        || o.outer
            .iter()
            .any(|p| inside(z, *p) && !o.holes.iter().any(|h| inside(h, *p)))
}

/// Whether two rings' boxes meet: the cheap test before the exact one.
fn boxes_meet(a: &[V], b: &[V]) -> bool {
    let bounds = |r: &[V]| {
        r.iter()
            .fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |m, p| {
                [m[0].min(p.x), m[1].min(p.y), m[2].max(p.x), m[3].max(p.y)]
            })
    };
    let (p, q) = (bounds(a), bounds(b));
    p[0] <= q[2] && q[0] <= p[2] && p[1] <= q[3] && q[1] <= p[3]
}

/// `ring` shrunk by [`SLACK`] toward its middle.
fn shrunk(ring: &[V]) -> Vec<V> {
    let n = ring.len() as f64;
    let c = V::new(
        ring.iter().map(|p| p.x).sum::<f64>() / n,
        ring.iter().map(|p| p.y).sum::<f64>() / n,
    );
    ring.iter()
        .map(|p| {
            let (dx, dy) = (p.x - c.x, p.y - c.y);
            let d = (dx * dx + dy * dy).sqrt();
            if d <= SLACK {
                c
            } else {
                V::new(p.x - dx / d * SLACK, p.y - dy / d * SLACK)
            }
        })
        .collect()
}

fn middle(ring: &[V]) -> V {
    let n = ring.len().max(1) as f64;
    V::new(
        ring.iter().map(|p| p.x).sum::<f64>() / n,
        ring.iter().map(|p| p.y).sum::<f64>() / n,
    )
}

/// A door's zones: its zones as it opens and its use zone, with its slot, its host's slot, its mark, how it opens, and
/// what it is drawn from (its jambs and spec), so its zones turned can be tried.
struct Door {
    slot: Slot,
    level: Option<String>,
    host: Option<Slot>,
    mark: String,
    operation: OpeningOperation,
    place: Vec<V>,
    spec: opening::Spec,
    sweeps: Vec<Vec<V>>,
    zone: Option<Vec<V>>,
}

/// How a door can be turned (`cad.openings.edit`'s `flip`), in the order a fix tries them: its words, its `what`.
pub const TURNS: [(&str, &str); 3] = [
    ("Açılışı öbür yüze çevir", "side"),
    ("Menteşe yanını çevir", "hand"),
    ("Açılışı ve menteşe yanını çevir", "both"),
];

/// A door's clash words by how it opens (docs/adr/0787 §1): what of it strikes, and what to do.
fn clash_words(d: &Door, what: &str) -> (String, String) {
    match d.operation {
        OpeningOperation::Revolving => (
            format!("{} döner kapısının tamburu bir {} çarpıyor (fiziksel çarpışma).", d.mark, dative(what)),
            "Döner kapıyı kaydırın, tamburunu küçültün ya da engeli taşıyın.".into(),
        ),
        OpeningOperation::Folding => (
            format!("{} katlanır kapısının panelleri toplanırken bir {} çarpıyor (fiziksel çarpışma).", d.mark, dative(what)),
            "Kapının açılış yönünü ya da toplandığı yanı çevirin, panel sayısını artırın ya da engeli taşıyın.".into(),
        ),
        OpeningOperation::Slide => (
            format!("{} sürme kapısının kanadı kayarken bir {} çarpıyor (fiziksel çarpışma).", d.mark, dative(what)),
            "Kapının kaydığı yüzü ya da yönü çevirin ya da engeli taşıyın.".into(),
        ),
        _ => (
            format!("{} kapısının kanadı açılırken bir {} çarpıyor (fiziksel çarpışma).", d.mark, dative(what)),
            "Kapının açılış yönünü ya da menteşesini çevirin, kanadı daraltın ya da engeli taşıyın.".into(),
        ),
    }
}

/// The door's zones turned as `what` says.
fn turned(d: &Door, what: &str) -> Vec<Vec<V>> {
    let mut spec = d.spec.clone();
    if what != "hand" {
        spec.side_right = !spec.side_right;
    }
    if what != "side" {
        spec.hand_end = !spec.hand_end;
    }
    opening::door_zones(&d.place, &spec)
}

/// Whether `zones` clear every obstacle off the door's host and on its level, and every other door's zones.
fn clear_of(d: &Door, zones: &[Vec<V>], obstacles: &[Obstacle], doors: &[Door]) -> bool {
    zones.iter().all(|z| {
        let shrunk_z = shrunk(z);
        obstacles
            .iter()
            .filter(|o| Some(o.slot) != d.host && o.level == d.level)
            .all(|o| !overlaps(&shrunk_z, o))
            && doors
                .iter()
                .filter(|o| o.slot != d.slot && o.level == d.level)
                .all(|o| o.sweeps.iter().all(|s| !zones_meet(z, s)))
    })
}

/// Whether two zones overlap by more than a touch.
fn zones_meet(a: &[V], b: &[V]) -> bool {
    if !boxes_meet(a, b) {
        return false;
    }
    let o = Obstacle {
        slot: Slot(0),
        level: None,
        what: "",
        outer: shrunk(b),
        holes: Vec::new(),
    };
    overlaps(&shrunk(a), &o)
}

/// The first turn that leaves the door clear of everything, as its `what`; none when none does or its zones do not
/// change (a revolving door's drum).
fn fix_of(d: &Door, obstacles: &[Obstacle], doors: &[Door]) -> Option<(&'static str, &'static str)> {
    TURNS.into_iter().find(|(_, what)| {
        let zones = turned(d, what);
        zones != d.sweeps && clear_of(d, &zones, obstacles, doors)
    })
}

fn doors(doc: &Document) -> Vec<Door> {
    let mut out = Vec::new();
    for e in doc.entities() {
        let Entity::Opening(o) = e else { continue };
        let f = &o.opening;
        if f.form != OpeningForm::Door || f.place.len() != 4 || f.broken.is_some() {
            continue;
        }
        let place: Vec<V> = f.place.iter().map(|p| V::new(p.x, p.y)).collect();
        let spec = kentos_formats::openings::spec(f);
        let host = kentos_domain::Uuid::parse_str(&f.host)
            .ok()
            .and_then(|u| doc.slot_of(u));
        out.push(Door {
            slot: Slot(o.base.id),
            level: crate::opening_types::level_of(doc, f),
            host,
            mark: f
                .number
                .clone()
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| "numarasız".into()),
            operation: f.operation,
            sweeps: opening::door_zones(&place, &spec),
            zone: opening::use_zone(&place, &spec),
            place,
            spec,
        });
    }
    out
}

fn obstacles(doc: &Document) -> Vec<Obstacle> {
    let mut out = Vec::new();
    for e in doc.entities() {
        let slot = Slot(e.base().id);
        let level = e.own_level().map(str::to_owned);
        match e {
            Entity::Wall(w) => {
                if let Some((outer, holes)) = wall_rings(&w.wall) {
                    out.push(Obstacle {
                        slot,
                        level,
                        what: "duvar",
                        outer,
                        holes,
                    });
                }
            }
            Entity::Column(c) => {
                if let kentos_geometry_core::entity::Shape::Column { at, column } =
                    kentos_formats::arch_elements::core_column(&c.column)
                {
                    let ring = kentos_geometry_core::geom::arch::column_corners(at, &column);
                    if ring.len() >= 3 {
                        out.push(Obstacle {
                            slot,
                            level,
                            what: "kolon",
                            outer: ring,
                            holes: Vec::new(),
                        });
                    }
                }
            }
            Entity::Insert(i) => {
                if let Some(ring) = doc.fixture_footprint(i) {
                    out.push(Obstacle {
                        slot,
                        level,
                        what: "donatı",
                        outer: ring.iter().map(|p| V::new(p.x, p.y)).collect(),
                        holes: Vec::new(),
                    });
                }
            }
            _ => {}
        }
    }
    out
}

/// The door group's findings: clashes, blocked fronts and, given `min_clear`, doors narrower in clear.
/// A clash's fix comes as its words and its `flip`'s `what` (the caller makes the call, none on a locked layer).
pub(crate) type Turn = Option<(&'static str, &'static str)>;

pub(crate) fn findings(
    doc: &Document,
    min_clear: Option<f64>,
    mut finding: impl FnMut(&str, String, String, Vec<Slot>, V, ArchAuditNature, Turn) -> ArchAuditFinding,
) -> Vec<ArchAuditFinding> {
    let doors = doors(doc);
    let obstacles = obstacles(doc);
    let mut out = Vec::new();
    for (i, d) in doors.iter().enumerate() {
        // Its fix, tried once and only when it clashes.
        let mut fix: Option<Turn> = None;
        let mut fix_for = |d: &Door| *fix.get_or_insert_with(|| fix_of(d, &obstacles, &doors));
        for sweep in &d.sweeps {
            let z = shrunk(sweep);
            for o in &obstacles {
                if Some(o.slot) == d.host || o.level != d.level || !overlaps(&z, o) {
                    continue;
                }
                let (message, repair) = clash_words(d, o.what);
                out.push(finding(
                    crate::codes::ARCH_DOOR_SWING_CLASH,
                    message,
                    repair,
                    vec![d.slot, o.slot],
                    middle(sweep),
                    ArchAuditNature::Physical,
                    fix_for(d),
                ));
            }
            for (j, other) in doors.iter().enumerate() {
                if j <= i || other.level != d.level {
                    continue;
                }
                if other.sweeps.iter().any(|s| zones_meet(sweep, s)) {
                    let message = if d.operation == OpeningOperation::Swing && other.operation == OpeningOperation::Swing {
                        format!(
                            "{} ve {} kapılarının kanatları açılırken birbirine çarpıyor (fiziksel çarpışma).",
                            d.mark, other.mark
                        )
                    } else {
                        format!(
                            "{} ve {} kapıları açılırken birbirinin alanına giriyor (fiziksel çarpışma).",
                            d.mark, other.mark
                        )
                    };
                    out.push(finding(
                        crate::codes::ARCH_DOOR_SWING_CLASH,
                        message,
                        "Kapılardan birinin açılış yönünü ya da menteşesini çevirin ya da kapıyı kaydırın.".into(),
                        vec![d.slot, other.slot],
                        middle(sweep),
                        ArchAuditNature::Physical,
                        fix_for(d),
                    ));
                }
            }
        }
        if let Some(zone) = &d.zone {
            let z = shrunk(zone);
            for o in &obstacles {
                if Some(o.slot) == d.host || o.level != d.level || !overlaps(&z, o) {
                    continue;
                }
                out.push(finding(
                    crate::codes::ARCH_DOOR_FRONT_BLOCKED,
                    format!(
                        "{} kapısının önündeki kullanım alanına bir {} giriyor (net geçiş).",
                        d.mark, o.what
                    ),
                    "Engeli kullanım alanının dışına alın ya da kapıyı başka yere taşıyın.".into(),
                    vec![d.slot, o.slot],
                    middle(zone),
                    ArchAuditNature::Passage,
                    None,
                ));
            }
        }
        if let Some(min) = min_clear
            && let Some(Entity::Opening(o)) = doc.get(d.slot)
            && let Some(clear) = o.opening.sizes().clear
            && clear < min - 1e-9
        {
            out.push(finding(
                crate::codes::ARCH_DOOR_CLEAR_NARROW,
                format!(
                    "{} kapısının net geçişi {} m; proje kuralının en az {} m'sinden dar (proje kuralı).",
                    d.mark,
                    fixed(clear, 3),
                    fixed(min, 3)
                ),
                "Kapıyı genişletin, kasayı inceltin ya da kanadın açılma açısını büyütün.".into(),
                vec![d.slot],
                middle(&o.opening.place.iter().map(|p| V::new(p.x, p.y)).collect::<Vec<_>>()),
                ArchAuditNature::Rule,
                None,
            ));
        }
    }
    let _ = ArchAuditCheck::Doors;
    out
}

/// What an obstacle is, as a door strikes it: “bir duvara”.
fn dative(what: &str) -> &'static str {
    match what {
        "duvar" => "duvara",
        "kolon" => "kolona",
        _ => "donatıya",
    }
}
