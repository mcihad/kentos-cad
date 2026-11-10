//! Pafta başlığının çizimden okunanları (docs/adr/8037; M37): what the first sheet's table of Planlı Alanlar İmar
//! Yönetmeliği 57(10) and the MO title block read of the drawing itself, never typed on a sheet. A sheet reads them as
//! `@` values after the drawing's own variables, so a variable of the same name the user wrote wins.
//!
//! - From İmar durumu (docs/adr/8030): the parcel's area, the plan's emsal and TAKS, the floors' area the emsal
//!   counts (emsale konu alan), the footprint (taban alanı), and every storey's rooms (yapı inşaat alanı; a single
//!   building's is also the parcel's total).
//! - From the storeys: how many above ±0.00 and below it have rooms (kat adedi, bodrum kat adedi).
//! - From the building program (docs/adr/8031): the independent units and their uses (konut, ticaret, büro …).

use std::collections::BTreeSet;

use kentos_contracts::{Entity, UnitKind};

use crate::Document;
use crate::arch_values::ArchValue;
use crate::room_names::{dwelling_nature, has, words};

/// One value a title block reads of the drawing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TitleFact {
    /// Its `@` name.
    pub name: &'static str,
    pub label: &'static str,
    pub value: String,
}

/// A unit's use as a building's use reads (PAİY 57(10) “kullanım amacı”): a dwelling konut, a shop ticaret, an office
/// büro; another its own nature.
fn use_of(nature: Option<&str>) -> String {
    if dwelling_nature(nature) {
        return "Konut".into();
    }
    let w = words(nature, None);
    if ["DUKKAN", "MAGAZA", "MARKET", "BANKA"]
        .iter()
        .any(|p| has(&w, p))
    {
        "Ticaret".into()
    } else if ["BURO", "OFIS"].iter().any(|p| has(&w, p)) {
        "Büro".into()
    } else {
        nature.unwrap_or_default().trim().to_owned()
    }
}

impl Document {
    /// What the drawing says of itself for a title block (docs/adr/8037), each with its `@` name; a value the drawing
    /// does not hold (no İmar durumu, no building program) is left out, so the sheet says it is missing.
    pub fn title_facts(&self) -> Vec<TitleFact> {
        let mut out = Vec::new();
        let mut put = |name: &'static str, label: &'static str, value: String| {
            out.push(TitleFact { name, label, value });
        };
        let area = |v: f64| self.arch_value_text(&ArchValue::Area(v), true);
        let ratio = |v: f64| crate::text_fields::number_text(v, 1.0, 2, false);
        if let Some(r) = self.zoning_report() {
            put("arsa_alani", "Arsa alanı", area(r.parcel));
            if let Some(f) = r.far_limit {
                put("emsal_orani", "Emsal (plan)", ratio(f));
            }
            if let Some(c) = r.coverage_limit {
                put("taks_orani", "TAKS (plan)", ratio(c));
            }
            put("emsale_konu_alan", "Emsale konu alan", area(r.floors));
            put("taban_alani", "Taban alanı", area(r.footprint));
            let construction: f64 = r.levels.iter().map(|l| l.all()).sum::<f64>() + 0.0;
            put("yapi_insaat_alani", "Yapı inşaat alanı", area(construction));
            put(
                "toplam_yapi_insaat_alani",
                "Toplam yapı inşaat alanı",
                area(construction),
            );
        }
        // The storeys with rooms, above and below ±0.00.
        let levels = self
            .settings()
            .arch
            .as_ref()
            .map(|a| a.levels.clone())
            .unwrap_or_default();
        if !levels.is_empty() {
            let used: BTreeSet<String> = self
                .entities()
                .filter_map(|e| match e {
                    Entity::Space(s) if !s.space.void => s.space.arch.level.clone(),
                    _ => None,
                })
                .collect();
            let (mut above, mut below) = (0, 0);
            for l in levels.iter().filter(|l| used.contains(&l.id)) {
                if l.elevation < -1e-6 {
                    below += 1;
                } else {
                    above += 1;
                }
            }
            if above + below > 0 {
                put("kat_adedi", "Kat adedi (bodrum hariç)", above.to_string());
                put("bodrum_kat_adedi", "Bodrum kat adedi", below.to_string());
            }
        }
        let program = self.program();
        let units: Vec<_> = program
            .units
            .iter()
            .filter(|u| u.kind == UnitKind::Unit)
            .collect();
        if !units.is_empty() {
            put(
                "bb_sayisi",
                "Bağımsız bölüm sayısı",
                units.len().to_string(),
            );
            let mut uses: Vec<String> = Vec::new();
            for u in &units {
                let w = use_of(u.nature.as_deref());
                if !w.is_empty() && !uses.contains(&w) {
                    uses.push(w);
                }
            }
            if !uses.is_empty() {
                put("kullanim_amaci", "Kullanım amacı", uses.join(" + "));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_unit_s_use_is_said_as_a_building_s() {
        assert_eq!(use_of(Some("Daire")), "Konut");
        assert_eq!(use_of(None), "Konut");
        assert_eq!(use_of(Some("Dükkân")), "Ticaret");
        assert_eq!(use_of(Some("Banka şubesi")), "Ticaret");
        assert_eq!(use_of(Some("Büro")), "Büro");
        assert_eq!(use_of(Some("Otel")), "Otel");
    }
}
