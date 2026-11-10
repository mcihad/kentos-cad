//! Mimari denetim's fixtures group (docs/adr/8025; ADV-12): the room in front of a fixture that a person using it
//! needs, its use zone, taken (net geçiş).
//!
//! - **The zone** is its block's “Boşluk zarfı” (docs/adr/0702: klozet önünde 80 × 60 cm, lavabo önünde 80 × 70 cm, duş
//!   girişinde 80 × 80 cm; `Document::fixture_zone`). KentOS's older sanitary blocks draw none (Klozet, Lavabo, Duş
//!   teknesi, their bytes kept for older projects): the same sizes stand in front of their body, across its middle.
//! - **Findings:** a wall, a column or another fixture in the zone is `arch_fixture_zone_blocked`; a door's leaf
//!   opening into it `arch_fixture_zone_door`, whose fix turns the door (`cad.openings.edit`'s Çevir) when that clears
//!   the zone and everything the door group checks. A touch is no finding (each zone shrunk by `door_audit::SLACK`).
//! - **Not said twice:** a fixture the building program gives a clearance is the program group's
//!   (`program_fixture_clearance`, docs/adr/0773 §5).
//!
//! The zones are a drawing aid, a common plan habit; no regulation is said to be met.

use kentos_commands::Entity;
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2 as V;
use kentos_geometry_core::display::fixed;

use crate::door_audit::{self, TURNS};

/// KentOS's older sanitary blocks' zones, by ADR 0702's sizes: the block's name, the zone's width and depth (metres).
pub const OLDER: [(&str, f64, f64); 3] = [
    ("Klozet", 0.80, 0.60),
    ("Lavabo", 0.80, 0.70),
    ("Duş teknesi", 0.80, 0.80),
];

/// A fixture with a use zone: its object, level, block's name, the zone and its width × depth.
pub struct Fixture {
    pub slot: Slot,
    pub level: Option<String>,
    pub name: String,
    pub zone: Vec<V>,
    pub size: (f64, f64),
}

fn v(p: kentos_commands::Vec2) -> V {
    V::new(p.x, p.y)
}

/// An older sanitary block's zone: `OLDER`'s size in front of its body (its +y side), across its middle, as the insert
/// places it.
fn older_zone(doc: &Document, i: &kentos_commands::InsertEntity, name: &str) -> Option<[V; 4]> {
    let key = kentos_commands::blocks::name_key(name);
    let (_, width, depth) = OLDER
        .iter()
        .find(|(n, ..)| kentos_commands::blocks::name_key(n) == key)?;
    let [x0, _, x1, y1] = doc.block_box(i.block)?;
    let (sx, sy) = (
        i.scale.abs().max(1e-12),
        i.y_scale.unwrap_or(i.scale).abs().max(1e-12),
    );
    // The zone is in the drawing's lengths: in the block's own frame it is divided by the insert's scale.
    let (half, deep) = (width / 2.0 / sx, depth / sy);
    let cx = (x0 + x1) / 2.0;
    let m = kentos_geometry_core::block::placement_xy(
        V::new(i.p.x, i.p.y),
        i.scale,
        i.y_scale.unwrap_or(i.scale),
        i.rotation,
        i.mirror,
    );
    let at = |x: f64, y: f64| V::new(m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]);
    Some([
        at(cx - half, y1),
        at(cx + half, y1),
        at(cx + half, y1 + deep),
        at(cx - half, y1 + deep),
    ])
}

/// The drawing's fixtures with a use zone, the building program's own left to it.
pub fn fixtures(doc: &Document) -> Vec<Fixture> {
    let program = doc.program();
    let mut out = Vec::new();
    for e in doc.entities() {
        let Entity::Insert(i) = e else { continue };
        let Some(b) = doc.blocks().iter().find(|b| b.id == i.block) else {
            continue;
        };
        if program.clearance(&b.name).is_some() {
            continue;
        }
        let zone = match doc.fixture_zone(i) {
            Some(z) => z.map(v),
            None => match older_zone(doc, i, &b.name) {
                Some(z) => z,
                None => continue,
            },
        };
        let d = |a: V, b: V| (b.x - a.x).hypot(b.y - a.y);
        let size = (d(zone[0], zone[1]), d(zone[1], zone[2]));
        out.push(Fixture {
            slot: Slot(i.base.id),
            level: e.own_level().map(str::to_owned),
            name: b.name.clone(),
            zone: zone.to_vec(),
            size,
        });
    }
    out
}

/// The fixtures group's findings. A door's fix comes as its words and its `flip`'s `what` (the caller makes the call,
/// none on a locked layer).
pub(crate) fn findings(
    doc: &Document,
    mut finding: impl FnMut(
        &str,
        String,
        String,
        Vec<Slot>,
        V,
        door_audit::Turn,
    ) -> kentos_commands::ArchAuditFinding,
) -> Vec<kentos_commands::ArchAuditFinding> {
    let fixtures = fixtures(doc);
    if fixtures.is_empty() {
        return Vec::new();
    }
    let obstacles = door_audit::obstacles(doc);
    let doors = door_audit::doors(doc);
    let named = |slot: Slot| match doc.get(slot) {
        Some(Entity::Insert(i)) => doc
            .blocks()
            .iter()
            .find(|b| b.id == i.block)
            .map(|b| b.name.clone()),
        _ => None,
    };
    let mut out = Vec::new();
    for f in &fixtures {
        let zone = door_audit::shrunk(&f.zone);
        let at = door_audit::middle(&f.zone);
        let size = format!("{} × {} m", fixed(f.size.0, 2), fixed(f.size.1, 2));
        for o in &obstacles {
            if o.slot == f.slot || o.level != f.level || !door_audit::overlaps(&zone, o) {
                continue;
            }
            let what = match (o.what, named(o.slot)) {
                ("donatı", Some(other)) => format!("başka bir donatı (“{other}”)"),
                (what, _) => format!("bir {what}"),
            };
            out.push(finding(
                crate::codes::ARCH_FIXTURE_ZONE_BLOCKED,
                format!("“{}” donatısının önündeki kullanım alanına ({size}) {what} giriyor (net geçiş).", f.name),
                "Donatıyı kaydırın ya da döndürün; önündeki alanı boşaltın.".into(),
                vec![f.slot, o.slot],
                at,
                None,
            ));
        }
        for d in &doors {
            if d.level != f.level || !d.sweeps.iter().any(|s| door_audit::zones_meet(&f.zone, s)) {
                continue;
            }
            // Its fix: the first turn whose leaf keeps out of the zone and clears what the door group checks.
            let turn = TURNS.into_iter().find(|(_, what)| {
                let zones = door_audit::turned(d, what);
                zones.iter().all(|s| !door_audit::zones_meet(&f.zone, s))
                    && door_audit::clear_of(d, &zones, &obstacles, &doors)
            });
            out.push(finding(
                crate::codes::ARCH_FIXTURE_ZONE_DOOR,
                format!(
                    "{} kapısının kanadı “{}” donatısının önündeki kullanım alanına ({size}) açılıyor (net geçiş).",
                    d.mark, f.name
                ),
                "Kapının açılış yönünü ya da menteşesini çevirin ya da donatıyı kaydırın.".into(),
                vec![f.slot, d.slot],
                at,
                turn,
            ));
        }
    }
    out
}
