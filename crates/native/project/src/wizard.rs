//! What the Yeni proje wizard asks, as data (docs/adr/0165 §3; the web's
//! `model/newProjectWizard.ts`): its choices, what each step offers for them,
//! what the rail and the summary say, and the project they make. The
//! desktop's window (apps/desktop/src/project/wizard) draws it; the web
//! records its answers for a few drafts (fixtures/project/v1/wizard.json)
//! and the test below gives the same here.

use kentos_contracts::{DrawingFont, DrawingUnit, Workspace};

use crate::crs::{LOCAL_SRID, System, system, systems, turef_zone_for};
use crate::new_project::{NEW_PROJECT_NAME, NewProject, js_number};
use crate::provinces::{Province, lower_tr, province};

/// The steps, in order: the project's type, its coordinates, its scale and details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Type,
    Coords,
    Details,
}

pub const STEPS: [Step; 3] = [Step::Type, Step::Coords, Step::Details];

impl Step {
    /// The step's name, as the rail writes it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Type => "Proje türü",
            Self::Coords => "Koordinatlar",
            Self::Details => "Ölçek ve ayrıntılar",
        }
    }

    /// Its place among the steps (0 for the first).
    pub fn index(self) -> usize {
        STEPS.iter().position(|s| *s == self).unwrap_or(0)
    }
}

/// The two types a wizard makes; the others are announced, not chosen yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Cad,
    Gis,
}

impl Kind {
    pub fn workspace(self) -> Workspace {
        match self {
            Self::Cad => Workspace::Cad,
            Self::Gis => Workspace::Gis,
        }
    }
}

/// A CAD project's coordinates: none (a drawing of its own, from 0,0) or a real coordinate system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Coords {
    Local,
    Real,
}

/// What the wizard holds while it is open; what is not chosen follows the others.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    #[serde(rename = "type")]
    pub kind: Kind,
    /// A CAD project's coordinates; a CBS project always has a system.
    pub coords: Coords,
    /// A local project's unit.
    pub unit: DrawingUnit,
    /// The province's plate code, whose centre the project opens on and whose zone is suggested.
    pub province: Option<u32>,
    /// The system chosen; none: the zone suggested for the province, else the app's default.
    pub srid: Option<u32>,
    /// The scale chosen (1:N); none: the type's own.
    pub plot_scale: Option<f64>,
    pub name: String,
    pub font: DrawingFont,
    /// The app's default system for new projects (Uygulama ayarları → Yeni projeler).
    pub fallback_srid: u32,
}

/// Map scales for a CBS project, drawing scales for a CAD one (1:N).
pub const GIS_SCALES: [f64; 7] = [500.0, 1000.0, 2000.0, 5000.0, 10_000.0, 25_000.0, 50_000.0];
pub const CAD_SCALES: [f64; 10] = [1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0];

/// A unit a local project is drawn in, with what it is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitChoice {
    pub id: DrawingUnit,
    pub name: &'static str,
    pub mark: &'static str,
    pub note: &'static str,
}

pub const UNITS: [UnitChoice; 3] = [
    UnitChoice {
        id: DrawingUnit::Mm,
        name: "Milimetre",
        mark: "mm",
        note: "Makine, detay ve imalat çizimleri",
    },
    UnitChoice {
        id: DrawingUnit::Cm,
        name: "Santimetre",
        mark: "cm",
        note: "İç mekân, mobilya ve doğrama",
    },
    UnitChoice {
        id: DrawingUnit::M,
        name: "Metre",
        mark: "m",
        note: "Mimari plan, vaziyet ve altyapı",
    },
];

fn unit_choice(id: DrawingUnit) -> &'static UnitChoice {
    UNITS.iter().find(|u| u.id == id).unwrap_or(&UNITS[2])
}

/// The layers the project starts with, as the summary names them.
fn layers(kind: Kind) -> &'static str {
    match kind {
        Kind::Cad => "Teknik çizim: Çizim, Ölçü, Yazı, Tarama, Yardımcı, Eksen",
        Kind::Gis => "Kadastro paftası: Taslak, Kadastro, Ulaşım, Topografya, Jeodezi, Pafta",
    }
}

/// A scale as written: 1:25.000 with Turkish digit groups (the web's `scaleText`).
pub fn scale_text(n: f64) -> String {
    format!("1:{}", tr_number(n))
}

/// `n.toLocaleString('tr-TR')`: digits grouped by dots, at most three decimals after a comma.
fn tr_number(n: f64) -> String {
    let thousandths = (n.abs() * 1000.0).round();
    let whole = (thousandths / 1000.0).trunc();
    let fraction = thousandths - whole * 1000.0;
    let digits = js_number(whole);
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    if fraction > 0.0 {
        let f = format!("{:03}", fraction as u32);
        out.push(',');
        out.push_str(f.trim_end_matches('0'));
    }
    if n < 0.0 && thousandths > 0.0 {
        format!("-{out}")
    } else {
        out
    }
}

/// A system as a sentence names it (the web's `crsTitle`).
fn title(s: &System) -> String {
    if s.is_local() {
        "Yerel (koordinat sistemi yok)".to_owned()
    } else {
        format!("{} (EPSG:{})", s.name, s.srid)
    }
}

impl Draft {
    /// A new wizard's draft: the app's last type, default system, typeface and unit.
    pub fn initial(
        kind: Workspace,
        fallback_srid: u32,
        font: DrawingFont,
        unit: Option<DrawingUnit>,
    ) -> Self {
        Self {
            kind: if kind == Workspace::Cad {
                Kind::Cad
            } else {
                Kind::Gis
            },
            coords: Coords::Local,
            unit: unit.unwrap_or(DrawingUnit::M),
            province: None,
            srid: None,
            plot_scale: None,
            name: NEW_PROJECT_NAME.to_owned(),
            font,
            fallback_srid: if fallback_srid != LOCAL_SRID && system(fallback_srid).is_some() {
                fallback_srid
            } else {
                5256
            },
        }
    }

    /// Whether the project is a drawing of its own, with no coordinate system.
    pub fn is_local(&self) -> bool {
        self.kind == Kind::Cad && self.coords == Coords::Local
    }

    /// The province chosen, if any.
    pub fn province(&self) -> Option<&'static Province> {
        self.province.and_then(province)
    }

    /// The TUREF zone of the province's longitude, if a province is chosen.
    pub fn suggested_srid(&self) -> Option<u32> {
        self.province()
            .and_then(|p| turef_zone_for(p.lon))
            .map(|s| s.srid)
    }

    /// The project's system: the local one, or the one chosen, suggested or defaulted.
    pub fn srid(&self) -> u32 {
        if self.is_local() {
            LOCAL_SRID
        } else {
            self.srid
                .or_else(|| self.suggested_srid())
                .unwrap_or(self.fallback_srid)
        }
    }

    /// The scales offered for the draft.
    pub fn scales(&self) -> &'static [f64] {
        match self.kind {
            Kind::Cad => &CAD_SCALES,
            Kind::Gis => &GIS_SCALES,
        }
    }

    /// The project's scale: the one chosen, else 1:1 for a local drawing and 1:1000 for any other.
    pub fn scale(&self) -> f64 {
        self.plot_scale
            .unwrap_or(if self.is_local() { 1.0 } else { 1000.0 })
    }

    /// The project's unit: a local drawing's own, metres for any other.
    pub fn unit(&self) -> DrawingUnit {
        if self.is_local() {
            self.unit
        } else {
            DrawingUnit::M
        }
    }

    /// The systems a project with coordinates chooses from: the suggested
    /// one first, then every other in the list's order (the local one is
    /// chosen on its own card).
    pub fn system_choices(&self) -> Vec<(&'static System, bool)> {
        let suggested = self.suggested_srid();
        let all = systems().iter().filter(|s| !s.is_local());
        all.clone()
            .filter(|s| Some(s.srid) == suggested)
            .map(|s| (s, true))
            .chain(
                all.filter(|s| Some(s.srid) != suggested)
                    .map(|s| (s, false)),
            )
            .collect()
    }

    /// What the rail writes under a step's name: the choice made there.
    pub fn note(&self, step: Step) -> String {
        match step {
            Step::Type => match self.kind {
                Kind::Cad => "CAD · teknik çizim".to_owned(),
                Kind::Gis => "CBS · harita".to_owned(),
            },
            Step::Coords => {
                if self.is_local() {
                    return format!("Yerel · {}", unit_choice(self.unit).mark);
                }
                [
                    self.province().map(|p| p.name.as_str()),
                    system(self.srid()).map(|s| s.name.as_str()),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" · ")
            }
            Step::Details => format!("{} · {}", scale_text(self.scale()), self.shown_name()),
        }
    }

    fn shown_name(&self) -> &str {
        match self.name.trim() {
            "" => NEW_PROJECT_NAME,
            name => name,
        }
    }

    /// The summary of the last step, line by line.
    pub fn summary(&self) -> Vec<(&'static str, String)> {
        let local = self.is_local();
        let srid = self.srid();
        let p = self.province();
        let mut lines = vec![
            (
                "Tür",
                match self.kind {
                    Kind::Cad => "CAD, teknik çizim",
                    Kind::Gis => "CBS, coğrafi bilgi sistemi",
                }
                .to_owned(),
            ),
            (
                "Koordinatlar",
                if local {
                    "Yerel: koordinat sistemi yok, çizim 0,0’dan başlar".to_owned()
                } else {
                    system(srid).map_or_else(|| format!("EPSG:{srid}"), title)
                },
            ),
            ("Birim", lower_tr(unit_choice(self.unit()).name)),
        ];
        if let (false, Some(p)) = (local, p) {
            lines.push(("İl", p.name.clone()));
        }
        lines.push(("Ölçek", scale_text(self.scale())));
        lines.push(("Katmanlar", layers(self.kind).to_owned()));
        lines.push((
            "Açılış",
            if local {
                "A3 kâğıt, yatay; 0,0 sol altta".to_owned()
            } else if let Some(p) = p {
                format!("{} merkezinde bir pafta", p.name)
            } else {
                "Dilimin çalışma alanında bir pafta".to_owned()
            },
        ));
        lines
    }

    /// Why the step cannot be left forward, or none.
    pub fn blocked(&self, step: Step) -> Option<String> {
        match step {
            Step::Details if self.name.trim().is_empty() => {
                Some("Proje adı boş olamaz.".to_owned())
            }
            Step::Coords if !self.is_local() && system(self.srid()).is_none() => Some(format!(
                "EPSG:{} bu sürümde tanımlı değil; listeden bir sistem seçin.",
                self.srid()
            )),
            _ => None,
        }
    }

    /// The project the draft makes.
    pub fn options(&self) -> NewProject {
        let local = self.is_local();
        NewProject {
            name: self.name.clone(),
            srid: self.srid(),
            plot_scale: self.scale(),
            workspace: self.kind.workspace(),
            drawing_font: self.font,
            province: if local { None } else { self.province },
            drawing_unit: local.then_some(self.unit),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    const FIXTURE: &str = include_str!("../../../../fixtures/project/v1/wizard.json");

    /// The web's answers for its drafts (newProjectWizardFixture.ts).
    #[test]
    fn the_wizard_answers_as_the_web_does() {
        let file: Value = serde_json::from_str(FIXTURE).expect("the fixture reads");
        assert_eq!(file["format"], "kentos.new-project-wizard");
        let cases = file["cases"].as_array().expect("cases");
        assert!(cases.len() >= 7);
        for case in cases {
            let name = case["name"].as_str().expect("a name");
            let d: Draft = serde_json::from_value(case["draft"].clone()).expect("a draft");
            assert_eq!(Value::from(d.srid()), case["srid"], "{name}");
            assert_eq!(
                d.suggested_srid().map_or(Value::Null, Value::from),
                case["suggested"],
                "{name}"
            );
            assert_eq!(Some(d.scale()), case["scale"].as_f64(), "{name}");
            assert_eq!(
                serde_json::to_value(d.unit()).expect("a unit"),
                case["unit"],
                "{name}"
            );
            let notes: Vec<String> = STEPS.iter().map(|s| d.note(*s)).collect();
            assert_eq!(Value::from(notes), case["notes"], "{name}");
            let summary: Vec<Value> = d
                .summary()
                .into_iter()
                .map(|(k, v)| Value::from(vec![k.to_owned(), v]))
                .collect();
            assert_eq!(Value::from(summary), case["summary"], "{name}");
            assert_eq!(
                d.system_choices()
                    .first()
                    .map_or(Value::Null, |(s, _)| Value::from(s.srid)),
                case["firstSystem"],
                "{name}"
            );
            let o = &case["options"];
            let want = NewProject {
                name: o["name"].as_str().expect("a name").to_owned(),
                srid: o["srid"].as_u64().expect("a system") as u32,
                plot_scale: o["plotScale"].as_f64().expect("a scale"),
                workspace: serde_json::from_value(o["workspace"].clone()).expect("a type"),
                drawing_font: serde_json::from_value(o["drawingFont"].clone()).expect("a typeface"),
                province: o.get("province").and_then(Value::as_u64).map(|c| c as u32),
                drawing_unit: o
                    .get("drawingUnit")
                    .map(|u| serde_json::from_value(u.clone()).expect("a unit")),
            };
            assert_eq!(d.options(), want, "{name}");
        }
    }

    #[test]
    fn a_new_draft_starts_on_the_apps_choices_and_its_steps_say_what_stops_them() {
        let d = Draft::initial(Workspace::Plan3d, 0, DrawingFont::Arimo, None);
        // Only CAD and CBS are made yet; the local system is no default for a system.
        assert_eq!(
            (d.kind, d.fallback_srid, d.unit, d.font),
            (Kind::Gis, 5256, DrawingUnit::M, DrawingFont::Arimo)
        );
        let d = Draft::initial(
            Workspace::Cad,
            5254,
            DrawingFont::Barlow,
            Some(DrawingUnit::Mm),
        );
        assert!(d.is_local());
        assert_eq!(
            (d.srid(), d.scale(), d.unit()),
            (LOCAL_SRID, 1.0, DrawingUnit::Mm)
        );
        assert_eq!(d.blocked(Step::Coords), None);
        let real = Draft {
            coords: Coords::Real,
            srid: Some(1234),
            ..d.clone()
        };
        assert_eq!(
            real.blocked(Step::Coords).as_deref(),
            Some("EPSG:1234 bu sürümde tanımlı değil; listeden bir sistem seçin.")
        );
        let unnamed = Draft {
            name: "  ".into(),
            ..d
        };
        assert_eq!(
            unnamed.blocked(Step::Details).as_deref(),
            Some("Proje adı boş olamaz.")
        );
        assert_eq!(unnamed.note(Step::Details), "1:1 · Yeni proje");
        assert_eq!(STEPS.map(Step::index), [0, 1, 2]);
    }

    #[test]
    fn scales_are_written_in_turkish_groups() {
        assert_eq!(
            [1.0, 1000.0, 25_000.0, 1_000_000.0, 2.5, 0.125].map(scale_text),
            [
                "1:1",
                "1:1.000",
                "1:25.000",
                "1:1.000.000",
                "1:2,5",
                "1:0,125"
            ]
        );
    }
}
