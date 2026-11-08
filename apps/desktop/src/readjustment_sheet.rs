//! Ayırma çapı (docs/adr/2280; Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik m. 13(2)): the parts of the parcels
//! partly inside the boundary, with their boundaries and coordinates, from `cad.readjustment.scope`'s answer. The
//! window's Ayırma çapı view lists each part's corners (Y, X) beside its area. Çapı çizime yaz writes the parts as
//! areas on the layer “Ayırma çapı” (made when the drawing has none): their attributes are the parcel, the part and its
//! area. The write is one undo step “Ayırma çapı” through `cad.entities.create`.

use std::collections::BTreeMap;

use kentos_commands::{
    CommandResult, EntitiesCreate, EntityGeometry, NewObject, ReadjustmentScoped, RingGeometry, ScopePart, ScopeRing,
};
use kentos_commands::Vec2;
use kentos_domain::NewLayer;
use kentos_interaction::{Format, Level};
use kentos_native_application::{ExecutionContext, create};

use crate::app::App;

/// The layer the parts are written on, and the step's name.
pub const LAYER: &str = "Ayırma çapı";
/// The window's button.
pub const WRITE: &str = "Çapı çizime yaz";

/// A part's name: Giren, or Girmeyen with its number when the parcel has more than one part outside (each takes a
/// parcel number of its own, Article 13(3)).
pub fn part_name(parts: &[ScopePart], k: usize) -> String {
    let side = &parts[k].side;
    if side == "inside" {
        return "Giren".to_owned();
    }
    let outside: Vec<usize> = (0..parts.len()).filter(|&i| parts[i].side == "outside").collect();
    match outside.iter().position(|&i| i == k) {
        Some(n) if outside.len() > 1 => format!("Girmeyen {}", n + 1),
        _ => "Girmeyen".to_owned(),
    }
}

/// The view's rows: every corner of every part of every partial parcel, numbered within the part (its holes' corners
/// after its outer ring's, the part's name saying “delik”), the part's area on its first row.
pub fn rows(scope: &ReadjustmentScoped, f: &Format) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    for p in &scope.parcels {
        for (k, part) in p.parts.iter().enumerate() {
            let name = part_name(&p.parts, k);
            let mut n = 0;
            for (r, ring) in part.rings.iter().enumerate() {
                for c in &ring.pts {
                    n += 1;
                    out.push(vec![
                        p.parcel.clone(),
                        if r == 0 { name.clone() } else { format!("{name} (delik)") },
                        n.to_string(),
                        f.coord(c[0]),
                        f.coord(c[1]),
                        if n == 1 { f.area_bare(part.area) } else { String::new() },
                    ]);
                }
            }
        }
    }
    out
}

/// Whether the scope has a part to write.
pub fn has_parts(scope: &ReadjustmentScoped) -> bool {
    scope.parcels.iter().any(|p| !p.parts.is_empty())
}

fn ring_pts(r: &ScopeRing) -> Vec<Vec2> {
    r.pts.iter().map(|c| Vec2 { x: c[0], y: c[1] }).collect()
}

/// A part as an area to write: its rings, its parcel, its name and its area as written in the view.
fn new_object(parcel: &str, name: String, part: &ScopePart, f: &Format) -> Option<NewObject> {
    let outer = part.rings.first()?;
    let holes: Vec<RingGeometry> = part.rings[1..]
        .iter()
        .map(|h| RingGeometry { pts: ring_pts(h), bulges: h.bulges.clone(), zs: None })
        .collect();
    let attrs = BTreeMap::from([
        ("Parsel".to_owned(), parcel.to_owned()),
        ("Kısım".to_owned(), name),
        ("Alan (m²)".to_owned(), f.area_bare(part.area)),
    ]);
    Some(NewObject {
        geometry: EntityGeometry::Polygon {
            pts: ring_pts(outer),
            bulges: outer.bulges.clone(),
            holes: (!holes.is_empty()).then_some(holes),
            zs: None,
            parts: None,
        },
        color: None,
        line_weight: None,
        line_type: None,
        attrs: Some(attrs),
        label: None,
        symbol: None,
        label_of: None,
        label_scale: None,
        transparency: None,
        order: None,
    })
}

impl App {
    /// Çapı çizime yaz: the scope's parts as areas on “Ayırma çapı”, one undo step; the layer made when missing.
    pub(crate) fn readjustment_sheet_write(&mut self) {
        let f = self.format();
        let Some(Ok(scope)) = self.readjustment.scope.clone() else { return };
        let objects: Vec<NewObject> = scope
            .parcels
            .iter()
            .flat_map(|p| {
                let f = &f;
                p.parts.iter().enumerate().filter_map(move |(k, part)| new_object(&p.parcel, part_name(&p.parts, k), part, f))
            })
            .collect();
        if objects.is_empty() {
            self.say(Level::Info, format!("{LAYER}: kısmen giren parsel yok; yazılacak parça yok."));
            return;
        }
        let n = objects.len();
        let Some(doc) = self.document.as_mut() else { return };
        let model = &mut doc.model;
        let step = model.begin_group(LAYER);
        let existing = model
            .layers()
            .leaves()
            .into_iter()
            .find(|l| l.name == LAYER)
            .map(|l| l.id.clone());
        let layer = match existing {
            Some(id) => id,
            None => match model.add_layer(NewLayer::layer(LAYER), None, false) {
                Ok(id) => id,
                Err(refused) => {
                    model.cancel_group(step);
                    self.warn(refused.to_string());
                    return;
                }
            },
        };
        let input = EntitiesCreate { layer_id: layer, objects, operation: None, expected_revision: None };
        match create::execute(&mut ExecutionContext::new(model), input) {
            CommandResult::Completed { .. } => {
                model.end_group(step);
                self.say(Level::Success, format!("{LAYER}: {n} parça “{LAYER}” katmanına yazıldı (tek geri alma adımı)."));
            }
            other => {
                model.cancel_group(step);
                let why = match other {
                    CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                        error.message
                    }
                    _ => "Ayırma çapı yazılamadı.".to_owned(),
                };
                self.warn(why);
            }
        }
    }
}
