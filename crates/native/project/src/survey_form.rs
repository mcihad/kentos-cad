//! Proje ayarları › Ölçme's form (docs/adr/0169 §3): the project's survey
//! settings as eight texts (the refraction coefficient k, the two faces'
//! horizontal reading difference, the index error, the two faces' slope
//! distance difference; a traverse leg's two-way difference, a traverse's
//! angular and linear misclosure; the mean ellipsoidal height of the ground
//! values, docs/adr/0171 §2) and the texts read back. The height is typed
//! in metres. The tolerances'
//! angles are typed in cc in a gon project and in arc seconds in a degree
//! one and kept in radians; the lengths are typed in millimetres and kept in
//! metres. The
//! web's twin is `apps/web/src/model/surveyForm.ts`; both pass
//! `fixtures/project/v1/survey-form.json`, which
//! `scripts/fixtures/survey_form_cases.py` writes from the rules alone.

use std::f64::consts::PI;

use kentos_contracts::{AngleUnit, REFRACTION, SurveySettings};
use kentos_geometry_core::display::fixed;

use crate::definition_form::number;

/// What is said of a text that does not hold.
pub const NUMBER: &str = "Sayı yazın.";
pub const REFRACTION_RANGE: &str = "−1 ile 1 arasında bir sayı yazın; boş bırakılırsa 0.13.";
pub const TOLERANCE: &str = "Sıfırdan büyük bir sayı yazın; denetlenmeyecekse boş bırakın.";
pub const HEIGHT: &str =
    "−500 ile 9000 m arasında bir yükseklik yazın; zemin değerleri gerekmiyorsa boş bırakın.";

/// The form's fields, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Refraction,
    FaceHz,
    Index,
    FaceSlope,
    TwoWay,
    TraverseAngle,
    TraverseCoord,
    GroundHeight,
}

impl Field {
    pub const ALL: [Field; 8] = [
        Field::Refraction,
        Field::FaceHz,
        Field::Index,
        Field::FaceSlope,
        Field::TwoWay,
        Field::TraverseAngle,
        Field::TraverseCoord,
        Field::GroundHeight,
    ];

    /// The settings' key (`refraction`, `faceHz`, `index`, `faceSlope`).
    pub fn key(self) -> &'static str {
        match self {
            Field::Refraction => "refraction",
            Field::FaceHz => "faceHz",
            Field::Index => "index",
            Field::FaceSlope => "faceSlope",
            Field::TwoWay => "twoWay",
            Field::TraverseAngle => "traverseAngle",
            Field::TraverseCoord => "traverseCoord",
            Field::GroundHeight => "groundHeight",
        }
    }

    fn get(self, s: &SurveySettings) -> Option<f64> {
        match self {
            Field::Refraction => s.refraction,
            Field::FaceHz => s.face_hz,
            Field::Index => s.index,
            Field::FaceSlope => s.face_slope,
            Field::TwoWay => s.two_way,
            Field::TraverseAngle => s.traverse_angle,
            Field::TraverseCoord => s.traverse_coord,
            Field::GroundHeight => s.ground_height,
        }
    }

    fn set(self, s: &mut SurveySettings, v: f64) {
        let slot = match self {
            Field::Refraction => &mut s.refraction,
            Field::FaceHz => &mut s.face_hz,
            Field::Index => &mut s.index,
            Field::FaceSlope => &mut s.face_slope,
            Field::TwoWay => &mut s.two_way,
            Field::TraverseAngle => &mut s.traverse_angle,
            Field::TraverseCoord => &mut s.traverse_coord,
            Field::GroundHeight => &mut s.ground_height,
        };
        *slot = Some(v);
    }

    /// Whether the field is a length (typed in millimetres).
    fn length(self) -> bool {
        matches!(
            self,
            Field::FaceSlope | Field::TwoWay | Field::TraverseCoord
        )
    }

    /// A typed value as the settings keep it: cc × π / 2 000 000 and
    /// ″ × π / 648 000 rad, mm ÷ 1000 m.
    fn stored(self, v: f64, unit: AngleUnit) -> f64 {
        match (self, unit) {
            (Field::Refraction | Field::GroundHeight, _) => v,
            (f, _) if f.length() => v / 1000.0,
            (_, AngleUnit::Grad) => v * PI / 2_000_000.0,
            (_, AngleUnit::Deg) => v * PI / 648_000.0,
        }
    }

    /// A kept value in the unit it is typed in.
    fn typed(self, v: f64, unit: AngleUnit) -> f64 {
        match (self, unit) {
            (Field::Refraction | Field::GroundHeight, _) => v,
            (f, _) if f.length() => v * 1000.0,
            (_, AngleUnit::Grad) => v * 2_000_000.0 / PI,
            (_, AngleUnit::Deg) => v * 648_000.0 / PI,
        }
    }
}

/// The mark a tolerance's angle is typed with: cc (a ten-thousandth of a
/// gon) in a gon project, ″ in a degree one.
pub fn angle_mark(unit: AngleUnit) -> &'static str {
    match unit {
        AngleUnit::Grad => "cc",
        AngleUnit::Deg => "″",
    }
}

/// The display rule's four decimals without trailing zeros or a bare point.
fn trimmed(v: f64) -> String {
    let s = fixed(v, 4);
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.')
    } else {
        s.as_str()
    };
    if s.is_empty() || s == "-0" {
        "0".to_owned()
    } else {
        s.to_owned()
    }
}

/// The texts the form shows for the settings; an absent value (k's
/// default too) is an empty text.
pub fn texts(survey: Option<&SurveySettings>, unit: AngleUnit) -> [String; 8] {
    Field::ALL.map(|f| {
        survey
            .and_then(|s| f.get(s))
            .map(|v| trimmed(f.typed(v, unit)))
            .unwrap_or_default()
    })
}

/// The texts read: the settings of the values that hold (none: the
/// defaults), and what is said of each text that does not.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Read {
    pub survey: Option<SurveySettings>,
    pub problems: [Option<&'static str>; 8],
}

impl Read {
    /// Whether the form waits for a text to be put right.
    pub fn blocked(&self) -> bool {
        self.problems.iter().any(Option::is_some)
    }
}

/// Reads the form's texts in a project of `unit`.
pub fn read<S: AsRef<str>>(texts: &[S; 8], unit: AngleUnit) -> Read {
    let mut survey = SurveySettings::default();
    let mut problems = [None; 8];
    for (i, (f, t)) in Field::ALL.iter().zip(texts).enumerate() {
        let t = t.as_ref();
        if t.trim().is_empty() {
            continue;
        }
        let Some(v) = number(t).filter(|v| v.is_finite()) else {
            problems[i] = Some(NUMBER);
            continue;
        };
        if *f == Field::Refraction {
            if !SurveySettings::refraction_holds(v) {
                problems[i] = Some(REFRACTION_RANGE);
            } else if v != REFRACTION {
                f.set(&mut survey, v);
            }
            continue;
        }
        if *f == Field::GroundHeight {
            if SurveySettings::ground_height_holds(v) {
                f.set(&mut survey, v);
            } else {
                problems[i] = Some(HEIGHT);
            }
            continue;
        }
        let kept = f.stored(v, unit);
        if !(v > 0.0 && SurveySettings::tolerance_holds(kept)) {
            problems[i] = Some(TOLERANCE);
            continue;
        }
        f.set(&mut survey, kept);
    }
    Read {
        survey: (survey != SurveySettings::default()).then_some(survey),
        problems,
    }
}

/// The settings read from the texts with the reduction to the grid the
/// form keeps beside them (docs/adr/0171 §4): kept only with a height, as a
/// project keeps them (the web's `withReduction`).
pub fn with_reduction(survey: Option<SurveySettings>, reduce: bool) -> Option<SurveySettings> {
    SurveySettings {
        reduce_to_grid: reduce.then_some(true),
        ..survey.unwrap_or_default()
    }
    .sanitized()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn unit(v: &Value) -> AngleUnit {
        match v.as_str() {
            Some("deg") => AngleUnit::Deg,
            _ => AngleUnit::Grad,
        }
    }

    /// The settings as texts and the texts read back, as the shared cases
    /// say (the web reads the same file, `model/surveyForm.test.ts`).
    #[test]
    fn the_form_is_the_shared_cases() {
        let file: Value = serde_json::from_str(include_str!(
            "../../../../fixtures/project/v1/survey-form.json"
        ))
        .expect("the cases read");
        assert_eq!(file["format"], "kentos.survey-form");
        for (key, text) in [
            ("number", NUMBER),
            ("refraction", REFRACTION_RANGE),
            ("tolerance", TOLERANCE),
            ("height", HEIGHT),
        ] {
            assert_eq!(file["messages"][key], text, "{key}");
        }
        let keys: Vec<&str> = Field::ALL.iter().map(|f| f.key()).collect();
        assert_eq!(file["fields"], serde_json::json!(keys));
        for case in file["texts"].as_array().expect("texts") {
            let survey: Option<SurveySettings> =
                serde_json::from_value(case["survey"].clone()).expect("settings");
            let want: Vec<String> = serde_json::from_value(case["texts"].clone()).expect("texts");
            assert_eq!(
                texts(survey.as_ref(), unit(&case["unit"])).to_vec(),
                want,
                "{}",
                case["name"]
            );
        }
        // The reduction to the grid rides beside the texts, kept only with a height.
        let height = SurveySettings {
            ground_height: Some(850.0),
            ..SurveySettings::default()
        };
        assert_eq!(with_reduction(None, true), None);
        assert_eq!(
            with_reduction(Some(height.clone()), false),
            Some(height.clone())
        );
        assert_eq!(
            with_reduction(Some(height.clone()), true),
            Some(SurveySettings {
                reduce_to_grid: Some(true),
                ..height
            })
        );
        for case in file["reads"].as_array().expect("reads") {
            let typed: [String; 8] = serde_json::from_value(case["texts"].clone()).expect("texts");
            let got = read(&typed, unit(&case["unit"]));
            let want: Option<SurveySettings> =
                serde_json::from_value(case["survey"].clone()).expect("settings");
            assert_eq!(got.survey, want, "{}", case["name"]);
            for (f, problem) in Field::ALL.iter().zip(got.problems) {
                assert_eq!(
                    problem,
                    case["problems"][f.key()].as_str(),
                    "{} {}",
                    case["name"],
                    f.key()
                );
            }
        }
    }
}
