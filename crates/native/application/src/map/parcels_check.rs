//! `cad.parcels.check` v1 (plan/map-cad.md MAP-08, docs/adr/1400): Parsel alan denetimi, read only. Every parcel's
//! geometric area (the shared core's `entity_area`: arcs and holes included) beside its recorded area as written,
//! the difference and the tolerance of the 2022 regulation's Article 8(2) table for archive-approved geometries
//! ([`tolerance`]); the identity is İl, İlçe, Mahalle, Ada and Parsel together. Nothing is written: a computed area
//! never overwrites the record (CLAUDE.md §7, §23.1). The shared cases are `fixtures/commands/v1/cad.parcels.check.json`.

use std::collections::BTreeMap;

use kentos_commands::{
    CommandResult, Entity, PARCEL_AREA_RULE, PARCEL_LAYER, ParcelAreaCheck, ParcelsCheck, ParcelsChecked,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::entity::entity_area;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::geometry::shape;

/// The recorded area's attribute when none is given (`cad.parcels.create`'s).
pub const RECORDED_FIELD: &str = "Tapu alanı (m²)";

/// Article 8(2)'s table (Tapu Planlarında Yanılma Sınırının Belirlenmesi Hakkında Yönetmelik, RG 27.09.2022 / 31966),
/// increasing by slices: (the slice's first area, where its rate starts, the tolerance there, the rate). A fraction
/// between two slices goes by the lower one (100.75 m² by 11–100).
const SLICES: [(f64, f64, f64, f64); 7] = [
    (0.0, 0.0, 0.0, 0.05),
    (11.0, 10.0, 0.5, 0.02),
    (101.0, 100.0, 2.3, 0.01),
    (501.0, 500.0, 6.3, 0.005),
    (1001.0, 1000.0, 8.8, 0.004),
    (5001.0, 5000.0, 24.8, 0.003),
    (25001.0, 25000.0, 84.8, 0.0015),
];

/// The tolerance (square metres, either way) of a parcel of `area` square metres.
pub fn tolerance(area: f64) -> f64 {
    let (_, start, base, rate) = SLICES.iter().rev().find(|s| area >= s.0).copied().unwrap_or(SLICES[0]);
    base + (area - start) * rate
}

/// A recorded area that is a plain decimal (digits, at most one point or comma, digits: “118,5”, “723.525”), read;
/// none for any other text (“tapuda yok”, “1.250,50”).
pub fn recorded_value(text: &str) -> Option<f64> {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let plain = match text.split_once(['.', ',']) {
        Some((whole, fraction)) => digits(whole) && digits(fraction),
        None => digits(text),
    };
    plain.then(|| text.replace(',', ".").parse().ok()).flatten()
}

fn fail(code: &str, message: String, path: String) -> Stop {
    Stop::failed(error(code, message, Some(path)))
}

fn count(n: usize) -> u32 {
    n.min(u32::MAX as usize) as u32
}

fn run(doc: &Document, input: &ParcelsCheck) -> Result<ParcelsChecked, Stop> {
    let field = input.field.as_deref().map(str::trim).filter(|f| !f.is_empty()).unwrap_or(RECORDED_FIELD).to_owned();
    let parcels: Vec<&Entity> = match &input.uids {
        Some(uids) => {
            let mut out = Vec::new();
            for (i, uid) in uids.iter().enumerate() {
                let path = format!("uids[{i}]");
                if !checks::is_uid_text(uid) {
                    return Err(fail(codes::INVALID_UID, format!("“{uid}” geçerli bir nesne kimliği değil."), path));
                }
                let Some(e) = Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s)) else {
                    return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{uid}” kimlikli nesne çizimde yok."), path));
                };
                if !matches!(e, Entity::Polygon(_)) {
                    return Err(fail(
                        codes::NOT_A_PARCEL,
                        format!("“{uid}” bir alan değil; parsel alan denetimi yalnız alanları denetler."),
                        path,
                    ));
                }
                out.push(e);
            }
            out
        }
        None => doc.by_layer(PARCEL_LAYER).filter(|e| matches!(e, Entity::Polygon(_))).collect(),
    };
    let mut checked = Vec::new();
    for e in parcels {
        let attr = |k: &str| e.base().attrs.get(k).map(|v| v.trim().to_owned()).unwrap_or_default();
        let area = entity_area(&shape(e)).unwrap_or(0.0);
        let text = e.base().attrs.get(&field).map(|v| v.trim().to_owned()).filter(|v| !v.is_empty());
        let recorded = text.as_deref().and_then(recorded_value);
        let difference = recorded.map(|r| area - r);
        let limit = tolerance(area);
        let status = match (&text, difference) {
            (None, _) => "noRecord",
            (Some(_), None) => "invalidRecord",
            (Some(_), Some(d)) if d.abs() <= limit => "within",
            _ => "outside",
        };
        let (ada, parsel) = (attr("Ada"), attr("Parsel"));
        checked.push(ParcelAreaCheck {
            uid: doc.uid(kentos_domain::Slot(e.base().id)).map(|u| u.to_string()).unwrap_or_default(),
            il: attr("İl"),
            ilce: attr("İlçe"),
            mahalle: attr("Mahalle"),
            no_identity: ada.is_empty() || parsel.is_empty(),
            ada,
            parsel,
            area,
            recorded_text: text,
            recorded,
            difference,
            tolerance: limit,
            status: status.to_owned(),
            duplicate: false,
        });
    }
    // The identity is the place and the numbers together: Parsel 5 of Ada 101 and of Ada 102 are two parcels.
    let key = |p: &ParcelAreaCheck| format!("{}/{}/{}/{}/{}", p.il, p.ilce, p.mahalle, p.ada, p.parsel);
    let mut held: BTreeMap<String, usize> = BTreeMap::new();
    for p in checked.iter().filter(|p| !p.no_identity) {
        *held.entry(key(p)).or_default() += 1;
    }
    let mut duplicates = Vec::new();
    for p in checked.iter_mut().filter(|p| !p.no_identity) {
        let k = key(p);
        if held.get(&k).is_some_and(|n| *n > 1) {
            p.duplicate = true;
            if !duplicates.contains(&k) {
                duplicates.push(k);
            }
        }
    }
    let by = |s: &str| count(checked.iter().filter(|p| p.status == s).count());
    Ok(ParcelsChecked {
        rule: PARCEL_AREA_RULE.to_owned(),
        field,
        within: by("within"),
        outside: by("outside"),
        no_record: by("noRecord"),
        invalid_record: by("invalidRecord"),
        no_identity: count(checked.iter().filter(|p| p.no_identity).count()),
        duplicates,
        parcels: checked,
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ParcelsCheck) -> CommandResult<()> {
    match run(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &ParcelsCheck) -> CommandResult<ParcelsChecked> {
    match run(cx.doc, input) {
        Ok(output) => CommandResult::Completed { output, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ParcelsCheck) -> CommandResult<ParcelsChecked> {
    plan(cx, &input)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The slices meet: the table increases without a step, and the sources' worked examples come out.
    #[test]
    fn the_tolerance_follows_the_table() {
        for (a, t) in [(10.0, 0.5), (100.0, 2.3), (500.0, 6.3), (1000.0, 8.8), (5000.0, 24.8), (25000.0, 84.8)] {
            assert!((tolerance(a) - t).abs() < 1e-9, "{a}");
        }
        assert!((tolerance(950.0) - 8.55).abs() < 1e-9);
        assert!((tolerance(300.0) - 4.3).abs() < 1e-9);
        // A fraction between two slices by the lower one.
        assert!((tolerance(100.75) - (0.5 + 90.75 * 0.02)).abs() < 1e-12);
        assert_eq!(recorded_value("118,5"), Some(118.5));
        assert_eq!(recorded_value("723.525"), Some(723.525));
        assert_eq!(recorded_value("1.250,50"), None);
        assert_eq!(recorded_value("tapuda yok"), None);
    }
}
