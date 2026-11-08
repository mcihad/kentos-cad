//! Ayırma çapı (docs/adr/2280; Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik m. 13(2)): the parts of the parcels
//! partly inside the boundary, with their boundaries and coordinates, from `cad.readjustment.scope`'s answer. The
//! window's Ayırma çapı view lists each part's corners (their point numbers, Y, X) beside its area. Çapı çizime yaz writes the parts as
//! areas on the layer “Ayırma çapı” (made when the drawing has none): their attributes are the parcel, the part and its
//! area. The write is one undo step “Ayırma çapı” through `cad.entities.create`. Çapın paftaları makes a separation sheet
//! for each parcel through `cad.readjustment.sheets` (docs/adr/2680).

use std::collections::BTreeMap;

use iced::Task;
use kentos_commands::{
    CommandResult, EntitiesCreate, EntityGeometry, NewObject, ReadjustmentScoped, ReadjustmentSheets, RingGeometry,
    ScopePart, ScopeRing,
};
use kentos_commands::Vec2;
use kentos_domain::NewLayer;
use kentos_interaction::{Format, Level};
use kentos_native_application::{ExecutionContext, create, readjustment_sheets};

use crate::app::{App, Message};

/// The layer the parts are written on, and the step's name.
pub const LAYER: &str = "Ayırma çapı";
/// The window's button.
pub const WRITE: &str = "Çapı çizime yaz";
/// The window's button that makes a separation sheet for each parcel (docs/adr/2680).
pub const MAKE: &str = "Çapın paftaları";

/// A part's name: Giren and Girmeyen, numbered on a side with more than one (each piece outside takes a parcel number of
/// its own, Article 13(3)); the separation sheets name them alike (docs/adr/2680).
pub fn part_name(parts: &[ScopePart], k: usize) -> String {
    readjustment_sheets::part_names(parts).swap_remove(k)
}

/// The view's rows: every corner of every part of every partial parcel (its holes' corners after its outer ring's, the
/// part's name saying “delik”) with its point number on the separation sheet (the scope's, docs/adr/2680: a corner the
/// parts share has one number), the part's area on its first row.
pub fn rows(scope: &ReadjustmentScoped, f: &Format) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    for p in &scope.parcels {
        for (k, part) in p.parts.iter().enumerate() {
            let name = part_name(&p.parts, k);
            let mut first = true;
            for (r, ring) in part.rings.iter().enumerate() {
                for (i, c) in ring.pts.iter().enumerate() {
                    out.push(vec![
                        p.parcel.clone(),
                        if r == 0 { name.clone() } else { format!("{name} (delik)") },
                        ring.numbers.get(i).map_or_else(String::new, u32::to_string),
                        f.coord(c[0]),
                        f.coord(c[1]),
                        if first { f.area_bare(part.area) } else { String::new() },
                    ]);
                    first = false;
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
        line_pattern: None,
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
    /// Çapın paftaları (docs/adr/2680): `cad.readjustment.sheets` over the window's boundary, places, rounding and piece
    /// tolerance with KentOS's separation sheet; one undo step. The window closes and the sheets are shown.
    pub(crate) fn readjustment_sheets_make(&mut self) -> Task<Message> {
        let Some(scope) = self.readjustment_scope_input() else { return Task::none() };
        let input = ReadjustmentSheets { scope, source: None, options: serde_json::json!({}), expected_revision: None };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let result = readjustment_sheets::execute(&mut ExecutionContext::new(&mut doc.model), input);
        let Some(out) = self.answered(result) else { return Task::none() };
        let names: Vec<&str> = out.sheets.iter().map(|s| s.name.as_str()).collect();
        self.say(
            Level::Success,
            format!(
                "{MAKE}: {} pafta ({}); noktalar “{}” katmanlarında. Tek geri alma adımı.",
                out.sheets.len(),
                names.join(", "),
                readjustment_sheets::GROUP
            ),
        );
        self.readjustment = crate::readjustment_window::State::default();
        self.dialog = None;
        self.follow_sheet_book();
        self.update(Message::Sheet(kentos_sheet_ui::Message::Tab(1)))
    }

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
