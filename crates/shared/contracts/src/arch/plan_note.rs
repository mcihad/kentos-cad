//! Plan notları (docs/adr/8084): the values a zoning plan's notes give the measures the regulation reads, before the
//! regulations' own where PAİY lets a plan say more, and the plan's own limits PAİY points to.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// What a plan note gives a value for (docs/adr/8084): a measure a rule of the regulation reads, the plan's value
/// coming before the regulations' where PAİY lets a plan say more (69/2–3; 28/1: “uygulama imar planında daha fazla
/// belirlenmemiş ise”; 4/1-yyy: “İmar planlarında aksine bir hüküm bulunmaması halinde”, docs/adr/8158; 42/1 and the
/// roof's ridge and slope, docs/adr/8160; 41/1's projections and the basements, docs/adr/8162), or the plan's
/// own limit PAİY points to (5/11: “uygulama imar planında … gösterilen kat adedi veya bina yüksekliği aşılamaz”,
/// docs/adr/8098).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PlanNoteKey {
    /// Zemin kat yüksekliği, en çok (28/1): the ground storey's floor to floor, m.
    GroundStoreyHeight,
    /// Kat yüksekliği, en çok (28/1): every storey's on and over ±0.00 floor to floor (the ground storey's too, unless
    /// a note gives its own), m.
    StoreyHeight,
    /// Bina yüksekliği, en çok (5/11, 4/1-n; docs/adr/8098): from the building's ±0.00 to its eaves level (the top of
    /// its last storey's ceiling slab, 4/1-ppp), m.
    BuildingHeight,
    /// Kat adedi, en çok (5/11; docs/adr/8098): the ground and normal storeys, a whole number.
    Storeys,
    /// Subasman kotu, en çok (4/1-yyy; docs/adr/8158): the ground storey's floor over its building's ±0.00 at most, m
    /// (PAİY's +1.20 where the plan says nothing).
    PlinthMost,
    /// Subasman kotu, en az (4/1-yyy; docs/adr/8158): the ground storey's floor over its building's ±0.00 at least, m,
    /// below it with a minus (PAİY's ±0.00 where the plan says nothing).
    PlinthLeast,
    /// Mahya yüksekliği, en çok (40/4; docs/adr/8160): a sloping roof's ridge over the saçak seviyesi at most, m
    /// (PAİY's 5.00, the municipality's where it says one).
    RidgeMost,
    /// Çatı eğimi, en çok (40/2; docs/adr/8160): a roof's edges' slope at most, per cent (PAİY leaves it to the
    /// administration; the municipality's where it says one).
    RoofSlopeMost,
    /// Saçak genişliği, en çok (42/1; docs/adr/8160): a roof's overhang over its walls at most, m (PAİY's 1.00:
    /// “Uygulama imar planında belirlenmemiş ise”).
    EavesMost,
    /// Çıkma, en çok (41/1; docs/adr/8162): a projection past the setback line at most, m (PAİY's 1.50; 0: none).
    ProjectionMost,
    /// Bodrum kat adedi, en çok (docs/adr/8162): a building's storeys under its ground storey, a whole number.
    BasementsMost,
}

impl PlanNoteKey {
    pub const ALL: [PlanNoteKey; 11] = [
        PlanNoteKey::GroundStoreyHeight,
        PlanNoteKey::StoreyHeight,
        PlanNoteKey::BuildingHeight,
        PlanNoteKey::Storeys,
        PlanNoteKey::PlinthMost,
        PlanNoteKey::PlinthLeast,
        PlanNoteKey::RidgeMost,
        PlanNoteKey::RoofSlopeMost,
        PlanNoteKey::EavesMost,
        PlanNoteKey::ProjectionMost,
        PlanNoteKey::BasementsMost,
    ];

    /// As the form and a finding name it.
    pub fn label(self) -> &'static str {
        match self {
            PlanNoteKey::GroundStoreyHeight => "Zemin kat yüksekliği (en çok)",
            PlanNoteKey::StoreyHeight => "Kat yüksekliği (en çok)",
            PlanNoteKey::BuildingHeight => "Bina yüksekliği (en çok)",
            PlanNoteKey::Storeys => "Kat adedi (en çok)",
            PlanNoteKey::PlinthMost => "Subasman kotu (en çok)",
            PlanNoteKey::PlinthLeast => "Subasman kotu (en az)",
            PlanNoteKey::RidgeMost => "Mahya yüksekliği (en çok)",
            PlanNoteKey::RoofSlopeMost => "Çatı eğimi (en çok)",
            PlanNoteKey::EavesMost => "Saçak genişliği (en çok)",
            PlanNoteKey::ProjectionMost => "Çıkma (en çok)",
            PlanNoteKey::BasementsMost => "Bodrum kat adedi (en çok)",
        }
    }

    /// As the audit facts and the rules write it.
    pub fn name(self) -> &'static str {
        match self {
            PlanNoteKey::GroundStoreyHeight => "groundStoreyHeight",
            PlanNoteKey::StoreyHeight => "storeyHeight",
            PlanNoteKey::BuildingHeight => "buildingHeight",
            PlanNoteKey::Storeys => "storeys",
            PlanNoteKey::PlinthMost => "plinthMost",
            PlanNoteKey::PlinthLeast => "plinthLeast",
            PlanNoteKey::RidgeMost => "ridgeMost",
            PlanNoteKey::RoofSlopeMost => "roofSlopeMost",
            PlanNoteKey::EavesMost => "eavesMost",
            PlanNoteKey::ProjectionMost => "projectionMost",
            PlanNoteKey::BasementsMost => "basementsMost",
        }
    }

    /// The value's unit as the form writes it: metres, storeys for the storey count, per cent for a roof's slope.
    pub fn unit(self) -> &'static str {
        match self {
            PlanNoteKey::Storeys | PlanNoteKey::BasementsMost => "kat",
            PlanNoteKey::RoofSlopeMost => "%",
            _ => "m",
        }
    }

    /// Whether a note of it is the plan's own limit 5/11 reads (docs/adr/8098), not a storey's height 28/1 leaves to
    /// the plan.
    pub fn is_plan_limit(self) -> bool {
        matches!(self, PlanNoteKey::BuildingHeight | PlanNoteKey::Storeys)
    }

    /// Whether a note of it is the plan's subasman (docs/adr/8158): 4/1-yyy's range where the plan says otherwise.
    pub fn is_plinth(self) -> bool {
        matches!(self, PlanNoteKey::PlinthMost | PlanNoteKey::PlinthLeast)
    }

    /// Whether a note of it is about the roof (docs/adr/8160): its ridge, its slope or its eaves.
    pub fn is_roof(self) -> bool {
        matches!(self, PlanNoteKey::RidgeMost | PlanNoteKey::RoofSlopeMost | PlanNoteKey::EavesMost)
    }

    /// Whether a note of it is about the projections or the basements (docs/adr/8162).
    pub fn is_mass(self) -> bool {
        matches!(self, PlanNoteKey::ProjectionMost | PlanNoteKey::BasementsMost)
    }
}

/// The most a roof's slope plan note may give, per cent.
pub const PLAN_ROOF_SLOPE_MAX: f64 = 200.0;

/// The farthest a subasman plan note may stand from the building's ±0.00, either way, m.
pub const PLAN_PLINTH_MAX: f64 = 10.0;

/// The most a building height plan note may give, m.
pub const PLAN_BUILDING_HEIGHT_MAX: f64 = 1000.0;

/// The most storeys a storey count plan note may give.
pub const PLAN_STOREYS_MAX: f64 = 200.0;

/// A plan note's value (docs/adr/8084): what it is for, the value as the note writes it (metres for a height) and where
/// it stands (“UİP 12.3.2019, plan notu 7”), as the architect reads it from the plan: KentOS does not read a note's
/// words into a number.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlanNote {
    pub key: PlanNoteKey,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub value: f64,
    pub source: String,
}

/// The most plan notes a project keeps.
pub const PLAN_NOTES_MAX: usize = 50;

impl PlanNote {
    /// What is wrong with a project's plan notes: the field and the words. A storey's height finite, over 0 and at most
    /// 20 m, a building height up to 1000 m, a storey count a whole number from 1 to 200, a subasman within 10 m of
    /// ±0.00 and its least not above its most, a ridge over 0 and at most 20 m, a roof's slope over 0 and at most 200 %,
    /// eaves and a projection from 0 to 5 m, basements a whole number from 0 to 20; its source said (at most 200
    /// characters); one note for each measure.
    pub fn list_problem(notes: &[PlanNote]) -> Option<(String, String)> {
        if notes.len() > PLAN_NOTES_MAX {
            return Some(("planNotes".into(), format!("En çok {PLAN_NOTES_MAX} plan notu verilebilir.")));
        }
        for (i, n) in notes.iter().enumerate() {
            let wrong = match n.key {
                PlanNoteKey::GroundStoreyHeight | PlanNoteKey::StoreyHeight => {
                    (!(n.value.is_finite() && n.value > 0.0 && n.value <= 20.0)).then_some("sıfırdan büyük, en çok 20 m")
                }
                PlanNoteKey::BuildingHeight => (!(n.value.is_finite()
                    && n.value > 0.0
                    && n.value <= PLAN_BUILDING_HEIGHT_MAX))
                    .then_some("sıfırdan büyük, en çok 1000 m"),
                PlanNoteKey::Storeys => (!(n.value.is_finite()
                    && n.value.fract() == 0.0
                    && (1.0..=PLAN_STOREYS_MAX).contains(&n.value)))
                    .then_some("1 ile 200 arasında bir tam sayı"),
                PlanNoteKey::PlinthMost | PlanNoteKey::PlinthLeast => (!(n.value.is_finite()
                    && (-PLAN_PLINTH_MAX..=PLAN_PLINTH_MAX).contains(&n.value)))
                    .then_some("−10 ile +10 m arasında"),
                PlanNoteKey::RidgeMost => (!(n.value.is_finite() && n.value > 0.0 && n.value <= 20.0))
                    .then_some("sıfırdan büyük, en çok 20 m"),
                PlanNoteKey::RoofSlopeMost => (!(n.value.is_finite()
                    && n.value > 0.0
                    && n.value <= PLAN_ROOF_SLOPE_MAX))
                    .then_some("sıfırdan büyük, en çok %200"),
                PlanNoteKey::EavesMost | PlanNoteKey::ProjectionMost => {
                    (!(n.value.is_finite() && (0.0..=5.0).contains(&n.value))).then_some("0 ile 5 m arasında")
                }
                PlanNoteKey::BasementsMost => (!(n.value.is_finite()
                    && n.value.fract() == 0.0
                    && (0.0..=20.0).contains(&n.value)))
                    .then_some("0 ile 20 arasında bir tam sayı"),
            };
            if let Some(what) = wrong {
                return Some((
                    format!("planNotes.{i}.value"),
                    format!("“{}” plan notunun değeri {what} olmalı.", n.key.label()),
                ));
            }
            let source = n.source.trim();
            if source.is_empty() {
                return Some((
                    format!("planNotes.{i}.source"),
                    format!(
                        "“{}” plan notunun kaynağını yazın: planı ve notun numarası (UİP 12.3.2019, plan notu 7).",
                        n.key.label()
                    ),
                ));
            }
            if source.chars().count() > 200 {
                return Some((
                    format!("planNotes.{i}.source"),
                    format!("“{}” plan notunun kaynağı en çok 200 karakter olmalı.", n.key.label()),
                ));
            }
            if notes[..i].iter().any(|m| m.key == n.key) {
                return Some((
                    format!("planNotes.{i}.key"),
                    format!("“{}” için iki plan notu var; birini kaldırın.", n.key.label()),
                ));
            }
        }
        // The subasman's least above its most leaves no floor level (docs/adr/8158).
        let of = |k: PlanNoteKey| notes.iter().position(|n| n.key == k);
        if let (Some(most), Some(least)) = (of(PlanNoteKey::PlinthMost), of(PlanNoteKey::PlinthLeast))
            && notes[least].value > notes[most].value
        {
            return Some((
                format!("planNotes.{least}.value"),
                "“Subasman kotu (en az)” plan notunun değeri “Subasman kotu (en çok)” notununkini aşamaz.".into(),
            ));
        }
        None
    }
}
