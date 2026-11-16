//! The parcel's zoning and what KentOS Mimari measured against it (docs/adr/8030; docs/adr/8057 §92): the plan's
//! emsal (KAKS) and TAKS as the project's İmar durumu writes them, and the building's areas as the Emsal ve TAKS table
//! reads them (`kentos_domain::Document::zoning_report`): the floors' area counted for the emsal with article 5/8's 30 %
//! for the rooms out of it, and the footprint. What a rule of the national regulation or of a municipality's plan notes
//! (plan notları) compares, without measuring the drawing again.

use serde::{Deserialize, Serialize};

use super::super::{AREA, round};

/// The parcel's zoning and the building's areas against it, m² (PAİY 4/1-tt, 4/1-cccc, 5/5, 5/8, 21, 22).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ZoningFacts {
    /// Net imar parseli alanı, as the project's İmar durumu writes it (the ratios are over it, PAİY 5/5).
    pub parcel: f64,
    /// Emsal (KAKS) the plan gives; none when it gives none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub far: Option<f64>,
    /// TAKS the plan gives; none when it gives none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<f64>,
    /// Katlar alanı (emsale konu alan): the rooms counted in and the rooms out of it past the 30 % (5/8).
    pub floors: f64,
    /// The rooms counted in (emsale dahil).
    pub included: f64,
    /// Emsal harici, within the 30 % (22/1).
    pub exempt: f64,
    /// Emsal harici apart from the 30 % (5/8's own list: fire stairs and their halls, a shelter …).
    pub outside: f64,
    /// The 30 % of the parcel's emsal area the exempt rooms may take; none without an emsal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap: Option<f64>,
    /// The exempt rooms' area past the 30 %, counted in the floors' area.
    pub over: f64,
    /// Taban alanı: the ground storey's rooms counted (all rooms in a project without storeys).
    pub footprint: f64,
}

impl ZoningFacts {
    /// Rounded as every area of the facts: to 0.0001.
    pub fn normalized(mut self) -> ZoningFacts {
        for a in [
            &mut self.parcel,
            &mut self.floors,
            &mut self.included,
            &mut self.exempt,
            &mut self.outside,
            &mut self.over,
            &mut self.footprint,
        ] {
            *a = round(*a, AREA);
        }
        self.far = self.far.map(|f| round(f, AREA));
        self.coverage = self.coverage.map(|c| round(c, AREA));
        self.cap = self.cap.map(|c| round(c, AREA));
        self
    }
}
