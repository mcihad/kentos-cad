//! A map project's settings in the undo history (docs/adr/8054 §3): the work structure, the GNSS archive, the parcel
//! register and what follows it, the zoning plan.

use std::sync::Arc;

use crate::document::Document;

#[derive(Clone, Debug)]
pub(crate) enum MapOp {
    /// A map project's work structure (docs/adr/1200 §3), before and after.
    MapWork {
        before: std::sync::Arc<Option<kentos_contracts::MapWork>>,
        after: std::sync::Arc<Option<kentos_contracts::MapWork>>,
    },
    /// A map project's GNSS archive (docs/adr/1340 §2), before and after.
    MapSurvey {
        before: std::sync::Arc<Option<kentos_contracts::MapSurvey>>,
        after: std::sync::Arc<Option<kentos_contracts::MapSurvey>>,
    },
    /// The parcel register (docs/adr/4220), before and after.
    ParcelRegister {
        before: std::sync::Arc<Option<kentos_contracts::ParcelRegister>>,
        after: std::sync::Arc<Option<kentos_contracts::ParcelRegister>>,
    },
    /// The adjustments the project keeps (docs/adr/5200), before and after.
    SurveyAdjustments {
        before: std::sync::Arc<Vec<kentos_contracts::KeptAdjustment>>,
        after: std::sync::Arc<Vec<kentos_contracts::KeptAdjustment>>,
    },
    /// The reviews the project keeps of its monitoring jobs (docs/adr/5220), before and after.
    MonitoringReviews {
        before: std::sync::Arc<Vec<kentos_contracts::MonitoringReview>>,
        after: std::sync::Arc<Vec<kentos_contracts::MonitoringReview>>,
    },
    /// A zoning plan's kept symbol choices and plan change drafts (docs/adr/5040), before and after.
    ZoningPlan {
        before: std::sync::Arc<Option<kentos_contracts::ZoningPlanData>>,
        after: std::sync::Arc<Option<kentos_contracts::ZoningPlanData>>,
    },
}

impl MapOp {
    pub(crate) fn inverse(&self) -> Self {
        match self {
            MapOp::MapWork { before, after } => MapOp::MapWork {
                before: after.clone(),
                after: before.clone(),
            },
            MapOp::MapSurvey { before, after } => MapOp::MapSurvey {
                before: after.clone(),
                after: before.clone(),
            },
            MapOp::ParcelRegister { before, after } => MapOp::ParcelRegister {
                before: after.clone(),
                after: before.clone(),
            },
            MapOp::SurveyAdjustments { before, after } => MapOp::SurveyAdjustments {
                before: after.clone(),
                after: before.clone(),
            },
            MapOp::MonitoringReviews { before, after } => MapOp::MonitoringReviews {
                before: after.clone(),
                after: before.clone(),
            },
            MapOp::ZoningPlan { before, after } => MapOp::ZoningPlan {
                before: after.clone(),
                after: before.clone(),
            },
        }
    }

    pub(crate) fn apply(&self, doc: &mut Document) {
        match self {
            MapOp::MapWork { after, .. } => {
                Arc::make_mut(&mut doc.settings).map_work = after.as_ref().clone();
            }
            MapOp::MapSurvey { after, .. } => {
                Arc::make_mut(&mut doc.settings).map_survey = after.as_ref().clone();
            }
            MapOp::ParcelRegister { after, .. } => {
                Arc::make_mut(&mut doc.settings).parcel_register = after.as_ref().clone();
            }
            MapOp::SurveyAdjustments { after, .. } => {
                Arc::make_mut(&mut doc.settings).survey_adjustments = after.as_ref().clone();
            }
            MapOp::MonitoringReviews { after, .. } => {
                Arc::make_mut(&mut doc.settings).monitoring_reviews = after.as_ref().clone();
            }
            MapOp::ZoningPlan { after, .. } => {
                Arc::make_mut(&mut doc.settings).zoning_plan = after.as_ref().clone();
            }
        }
    }
}
