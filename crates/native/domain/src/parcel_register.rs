//! The parcel register's undo step (plan/map-cad.md MAP-08, docs/adr/4220). The register lives in the project's
//! settings, which are otherwise outside the model's undo; a document written to it is a step of its own.

use std::sync::Arc;

use kentos_contracts::{KeptAdjustment, MonitoringReview, ParcelRegister};

use crate::history::MapOp;
use crate::{Document, history::Op};

impl Document {
    /// The drawing's parcel register; none in a drawing without one.
    pub fn parcel_register(&self) -> Option<&ParcelRegister> {
        self.settings.parcel_register.as_ref()
    }

    /// Replaces the register as one undo step named `label` (inside an open transaction or group, part of it); an
    /// empty one is none. Nothing happens when it is the same.
    pub fn set_parcel_register(&mut self, next: Option<ParcelRegister>, label: &str) {
        let next = next.filter(|r| !r.is_empty());
        if self.settings.parcel_register == next {
            return;
        }
        let before = Arc::new(std::mem::replace(
            &mut Arc::make_mut(&mut self.settings).parcel_register,
            next,
        ));
        let after = Arc::new(self.settings.parcel_register.clone());
        self.record(
            vec![Op::Map(MapOp::ParcelRegister { before, after })],
            label,
        );
    }

    /// The adjustments the project keeps (docs/adr/5200).
    pub fn survey_adjustments(&self) -> &[KeptAdjustment] {
        &self.settings.survey_adjustments
    }

    /// Replaces the kept adjustments as one undo step named `label` (inside an open group, part of it). Nothing happens
    /// when they are the same.
    pub fn set_survey_adjustments(&mut self, next: Vec<KeptAdjustment>, label: &str) {
        if self.settings.survey_adjustments == next {
            return;
        }
        let before = Arc::new(std::mem::replace(
            &mut Arc::make_mut(&mut self.settings).survey_adjustments,
            next,
        ));
        let after = Arc::new(self.settings.survey_adjustments.clone());
        self.record(
            vec![Op::Map(MapOp::SurveyAdjustments { before, after })],
            label,
        );
    }

    /// The reviews the project keeps of its monitoring jobs (docs/adr/5220).
    pub fn monitoring_reviews(&self) -> &[MonitoringReview] {
        &self.settings.monitoring_reviews
    }

    /// Replaces the kept monitoring reviews as one undo step named `label`. Nothing happens when they are the same.
    pub fn set_monitoring_reviews(&mut self, next: Vec<MonitoringReview>, label: &str) {
        if self.settings.monitoring_reviews == next {
            return;
        }
        let before = Arc::new(std::mem::replace(
            &mut Arc::make_mut(&mut self.settings).monitoring_reviews,
            next,
        ));
        let after = Arc::new(self.settings.monitoring_reviews.clone());
        self.record(
            vec![Op::Map(MapOp::MonitoringReviews { before, after })],
            label,
        );
    }
}
